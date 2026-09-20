//! `#[job]` and `#[cron]`.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn, LitStr};

/// `500ms`, `30s`, `5m`, `2h`, `1d`, or combinations such as `1h30m`.
pub(crate) fn parse_duration_ms(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut total = 0u64;
    let mut rest = text;
    while !rest.is_empty() {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let n: u64 = rest[..digits].parse().ok()?;
        rest = &rest[digits..];
        let unit_len = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_alphabetic()).len();
        let unit = match &rest[..unit_len] {
            "ms" => 1,
            "s" => 1000,
            "m" => 60_000,
            "h" => 3_600_000,
            "d" => 86_400_000,
            _ => return None,
        };
        rest = &rest[unit_len..];
        total = total.checked_add(n.checked_mul(unit)?)?;
    }
    Some(total)
}

fn duration(lit: &LitStr) -> syn::Result<u64> {
    parse_duration_ms(&lit.value()).ok_or_else(|| {
        syn::Error::new(lit.span(), "expected a duration such as \"500ms\", \"30s\", \"5m\" or \"1h30m\"")
    })
}

fn is_job_context(arg: &FnArg) -> bool {
    matches!(arg, FnArg::Typed(a) if matches!(&*a.ty, syn::Type::Path(p)
        if p.path.segments.last().is_some_and(|s| s.ident == "JobContext")))
}

#[derive(Default)]
struct JobArgs {
    name: Option<LitStr>,
    queue: Option<LitStr>,
    max_attempts: Option<syn::Expr>,
    priority: Option<syn::Expr>,
    timeout: Option<u64>,
    backoff: Option<u64>,
    max_backoff: Option<u64>,
    path: Option<syn::Path>,
}

pub(crate) fn job(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut args = JobArgs::default();
    let parser = syn::meta::parser(|meta| {
        let key = meta.path.get_ident().map(ToString::to_string).unwrap_or_default();
        match key.as_str() {
            "name" => args.name = Some(meta.value()?.parse()?),
            "queue" => args.queue = Some(meta.value()?.parse()?),
            "max_attempts" => args.max_attempts = Some(meta.value()?.parse()?),
            "priority" => args.priority = Some(meta.value()?.parse()?),
            "timeout" => args.timeout = Some(duration(&meta.value()?.parse()?)?),
            "backoff" => args.backoff = Some(duration(&meta.value()?.parse()?)?),
            "max_backoff" => args.max_backoff = Some(duration(&meta.value()?.parse()?)?),
            "path" => args.path = Some(meta.value()?.parse::<LitStr>()?.parse()?),
            _ => {
                return Err(meta.error(
                    "unknown #[job] argument; expected name, queue, max_attempts, priority, timeout, backoff, \
                     max_backoff or path",
                ));
            }
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, attr)?;

    let mut func: ItemFn = syn::parse2(item)?;
    let ident = func.sig.ident.clone();
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new(ident.span(), "a #[job] must be an `async fn`"));
    }
    if !func.sig.generics.params.is_empty() {
        return Err(syn::Error::new(ident.span(), "a #[job] cannot be generic"));
    }
    if matches!(func.sig.inputs.first(), Some(FnArg::Receiver(_))) {
        return Err(syn::Error::new(ident.span(), "a #[job] cannot be a method"));
    }
    let inputs: Vec<&FnArg> = func.sig.inputs.iter().collect();
    // (input type, how `run` calls the function)
    let (input_ty, call) = match inputs.as_slice() {
        [] => (quote!(()), quote!(Self::perform())),
        [ctx] if is_job_context(ctx) => (quote!(()), quote!(Self::perform(__ctx))),
        [FnArg::Typed(input)] => {
            let ty = &input.ty;
            (quote!(#ty), quote!(Self::perform(__input)))
        }
        [FnArg::Typed(input), ctx] if is_job_context(ctx) => {
            let ty = &input.ty;
            (quote!(#ty), quote!(Self::perform(__input, __ctx)))
        }
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                "a #[job] takes nothing, one input, a JobContext, or (input, JobContext)",
            ));
        }
    };

    let krate = args.path.map(|p| quote!(#p)).unwrap_or_else(|| quote!(::next_rust::jobs));
    let name = args.name.unwrap_or_else(|| LitStr::new(&ident.to_string(), ident.span()));
    let queue = args.queue.map(|q| quote!(.queue(#q)));
    let max_attempts = args.max_attempts.map(|n| quote!(.max_attempts(#n)));
    let priority = args.priority.map(|p| quote!(.priority(#p)));
    let timeout = args.timeout.map(|ms| quote!(.timeout(::std::time::Duration::from_millis(#ms))));
    let backoff = (args.backoff.is_some() || args.max_backoff.is_some()).then(|| {
        let base = args.backoff.unwrap_or(5_000);
        let max = args.max_backoff.unwrap_or(3_600_000).max(base);
        quote!(.backoff(#krate::Backoff::exponential(
            ::std::time::Duration::from_millis(#base),
            ::std::time::Duration::from_millis(#max),
        )))
    });

    let vis = func.vis.clone();
    let attrs: Vec<syn::Attribute> = func.attrs.iter().filter(|a| a.path().is_ident("doc")).cloned().collect();
    func.sig.ident = syn::Ident::new("perform", ident.span());
    Ok(quote! {
        #(#attrs)*
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug)]
        #vis struct #ident;

        impl #ident {
            /// The job's body, callable directly (tests, synchronous fallbacks).
            #func
        }

        impl #krate::Job for #ident {
            type Input = #input_ty;
            const NAME: &'static str = #name;

            fn options() -> #krate::JobOptions {
                #krate::JobOptions::new() #queue #max_attempts #priority #timeout #backoff
            }

            #[allow(unused_variables)]
            async fn run(__input: Self::Input, __ctx: #krate::JobContext) -> ::std::result::Result<(), #krate::JobError> {
                #krate::IntoJobResult::into_job_result(#call.await)
            }
        }
    })
}

pub(crate) fn cron(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut schedule: Option<LitStr> = None;
    let mut name: Option<LitStr> = None;
    let mut path: Option<syn::Path> = None;
    let parser = |input: syn::parse::ParseStream| -> syn::Result<()> {
        schedule = Some(input.parse().map_err(|_| {
            syn::Error::new(Span::call_site(), "expected a cron expression: #[cron(\"0 */5 * * * *\")]")
        })?);
        while !input.is_empty() {
            input.parse::<syn::Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value: LitStr = input.parse()?;
            if key == "name" {
                name = Some(value);
            } else if key == "path" {
                path = Some(value.parse()?);
            } else {
                return Err(syn::Error::new(key.span(), "unknown #[cron] argument; expected name or path"));
            }
        }
        Ok(())
    };
    syn::parse::Parser::parse2(parser, attr)?;
    let Some(schedule) = schedule else {
        return Err(syn::Error::new(Span::call_site(), "expected a cron expression"));
    };
    // The full grammar is checked when the runner is built; catch the most
    // common slip (a wrong number of fields) at compile time.
    let text = schedule.value();
    let fields = text.split_whitespace().count();
    if !text.trim_start().starts_with('@') && fields != 5 && fields != 6 {
        return Err(syn::Error::new(
            schedule.span(),
            format!(
                "a cron expression has 5 fields (minute hour day month weekday) or 6 (seconds first); this one has {fields}"
            ),
        ));
    }

    let mut func: ItemFn = syn::parse2(item)?;
    let ident = func.sig.ident.clone();
    if func.sig.asyncness.is_none() {
        return Err(syn::Error::new(ident.span(), "a #[cron] task must be an `async fn`"));
    }
    if !func.sig.inputs.is_empty() || !func.sig.generics.params.is_empty() {
        return Err(syn::Error::new(ident.span(), "a #[cron] task takes no arguments and cannot be generic"));
    }
    let krate = path.map(|p| quote!(#p)).unwrap_or_else(|| quote!(::next_rust::jobs));
    let name = name.unwrap_or_else(|| LitStr::new(&ident.to_string(), ident.span()));
    let vis = func.vis.clone();
    let attrs: Vec<syn::Attribute> = func.attrs.iter().filter(|a| a.path().is_ident("doc")).cloned().collect();
    func.sig.ident = syn::Ident::new("perform", ident.span());
    Ok(quote! {
        #(#attrs)*
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug)]
        #vis struct #ident;

        impl #ident {
            /// The task's body, callable directly.
            #func
        }

        impl #krate::CronTask for #ident {
            const NAME: &'static str = #name;
            const SCHEDULE: &'static str = #schedule;

            async fn run() -> ::std::result::Result<(), #krate::JobError> {
                #krate::IntoJobResult::into_job_result(Self::perform().await)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration_ms("500ms"), Some(500));
        assert_eq!(parse_duration_ms("30s"), Some(30_000));
        assert_eq!(parse_duration_ms("1h30m"), Some(5_400_000));
        assert_eq!(parse_duration_ms("1d"), Some(86_400_000));
        for bad in ["", "5", "s", "5x", "5 s", "-5s"] {
            assert_eq!(parse_duration_ms(bad), None, "{bad}");
        }
    }

    #[test]
    fn job_expansion() {
        let out = job(
            quote!(queue = "mail", max_attempts = 3, timeout = "30s"),
            quote!(
                pub async fn send(input: Welcome, ctx: JobContext) -> Result<(), JobError> {
                    Ok(())
                }
            ),
        )
        .unwrap()
        .to_string();
        assert!(out.contains("pub struct send"), "{out}");
        assert!(out.contains("type Input = Welcome"), "{out}");
        assert!(out.contains("const NAME : & 'static str = \"send\""), "{out}");
        assert!(out.contains("from_millis (30000u64)"), "{out}");
        assert!(out.contains("Self :: perform (__input , __ctx)"), "{out}");
    }

    #[test]
    fn rejections() {
        assert!(
            job(
                quote!(),
                quote!(
                    fn sync(input: u8) {}
                )
            )
            .is_err()
        );
        assert!(
            job(
                quote!(timeout = "soon"),
                quote!(
                    async fn j() {}
                )
            )
            .is_err()
        );
        assert!(
            job(
                quote!(colour = "red"),
                quote!(
                    async fn j() {}
                )
            )
            .is_err()
        );
        assert!(
            job(
                quote!(),
                quote!(
                    async fn j(a: u8, b: u8, c: u8) {}
                )
            )
            .is_err()
        );
        assert!(
            cron(
                quote!("* * *"),
                quote!(
                    async fn t() {}
                )
            )
            .unwrap_err()
            .to_string()
            .contains("this one has 3")
        );
        assert!(
            cron(
                quote!("@daily"),
                quote!(
                    async fn t(x: u8) {}
                )
            )
            .is_err()
        );
        assert!(
            cron(
                quote!(),
                quote!(
                    async fn t() {}
                )
            )
            .is_err()
        );
        assert!(
            cron(
                quote!("0 3 * * *", name = "nightly"),
                quote!(
                    async fn t() {}
                )
            )
            .is_ok()
        );
    }
}
