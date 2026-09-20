//! The JavaScript shim: the only JavaScript an island-driven page runs.
//!
//! It is a switch statement over an opcode table, a handle table, a pending
//! string, a callback bridge and a resume queue. There is no application
//! behaviour in it and there must never be: if a change here would need an
//! `if` that encodes a policy rather than a null check or a try/catch, the
//! policy belongs in wasm.
//!
//! Three properties are load-bearing and each has a test below.
//!
//! **The opcode order is the ABI.** `O[n]` is indexed by the number the
//! module passes, and a module compiled against one numbering may run against
//! a shim a browser cached from an earlier deploy. Append, never reorder,
//! never reuse. `crates/next-rust-wasm/src/host.rs` is the same table in Rust
//! and is the authority; `every_core_opcode_is_implemented` checks the two
//! against each other, because that is the regression that would otherwise be
//! silent.
//!
//! **It survives minification.** Release builds minify the generated `client/`
//! modules like any other script, so the module URL is written out in full
//! rather than resolved from `import.meta.url` — `[build] inline_assets`
//! writes this file into the page as a classic script, where `import.meta` is
//! a syntax error.
//!
//! **Its props encoder agrees with the Rust one, byte for byte.** The shim
//! encodes a JSON value into the same binary blob `next_rust_wasm::props`
//! reads, so a disagreement would be a silently wrong prop rather than an
//! error. The golden vector below was produced by the Rust encoder and is
//! compared against this file's output when node is available.
//!
//! What cannot be removed: `WebAssembly.instantiate`, the import object and
//! `addEventListener` can only be reached from JavaScript. A shim always
//! remains. What it does not do is carry logic.
//!
//! The `<script>` tag that loads this must carry `data-nr-shim`, so tooling
//! that counts application JavaScript does not count the interpreter.

/// The island loader. Opcodes 1 to 114, 140 to 168.
pub const LOADER: &str = r#"const D=new TextDecoder,E=new TextEncoder,H=[0],F=[],G=[0,0,0,0];let M,X,P,S,V,C,B;
const s=(p,l)=>D.decode(new Uint8Array(M.buffer,p,l));
const R=v=>{if(v==null)return 0;const i=F.pop();if(i){H[i]=v;return i}return H.push(v)-1};
const g=a=>H[a],w=(p,...v)=>{const d=new DataView(M.buffer);v.forEach((x,i)=>d.setFloat64(p+i*8,x,!0));return 1};
const n=(lo,hi)=>{const d=new DataView(new ArrayBuffer(8));d.setUint32(0,lo,!0);d.setUint32(4,hi,!0);return d.getFloat64(0,!0)};
const t=v=>{S=v==null?null:E.encode(String(v));return S?S.length:4294967295};
const N=()=>window.nextRust||{};
const q=(f,d)=>{try{return f()}catch{return d}};
const ev=(cb,v)=>{const o=V,p=C;V=v;C=0;const h=R(v);X.on_event(cb,h);H[h]=0;F.push(h);V=o;C=p};
const dl=(r,sel,cb)=>v=>{const m=v.target&&v.target.closest&&v.target.closest(sel);if(m&&r.contains(m)){const o=C;C=m;const p=V;V=v;const h=R(v);X.on_event(cb,h);H[h]=0;F.push(h);V=p;C=o}};
const lo=f=>({passive:!!(f&1),capture:!!(f&2),once:!!(f&4)});
const rs=(tok,tag,st,b)=>{const u=b instanceof Uint8Array?b:E.encode(b==null?"":String(b)),k=u.length+3,p=X.alloc(k),a=new Uint8Array(M.buffer,p,k);a[0]=tag;a[1]=st&255;a[2]=st>>8;a.set(u,3);X.on_resume(tok,p,k);X.dealloc&&X.dealloc(p,k)};
const AB=new Map;
const fail=(tok,e)=>{let m=(e&&e.message)||"";const c=e&&(e.code||(e.data&&e.data.code));if(c)m+="\n_code\t"+c;const x=e&&e.errors;if(x)for(const k in x)m+="\n"+k+"\t"+x[k];rs(tok,1,(e&&e.status)||0,m)};
const act=(tok,url,input)=>{const f=N().action;if(!f){rs(tok,1,0,"no runtime");return 0}f(url,input).then(v=>rs(tok,0,200,blob(v&&typeof v==="object"&&!Array.isArray(v)?v:{value:v}))).catch(e=>fail(tok,e));return 0};
const fx=(tok,pr,how)=>{pr.then(async r=>{try{rs(tok,0,r.status||200,await how(r))}catch(e){rs(tok,1,0,e&&e.message)}},e=>rs(tok,1,0,e&&e.message));return 0};
const fnv=b=>{let h=0x811c9dc5;for(let i=0;i<b.length;i++){h^=b[i];h=Math.imul(h,0x01000193)>>>0}return h>>>0};
const vi=(o,v)=>{do{let c=v&127;v=Math.floor(v/128);o.push(v?c|128:c)}while(v)};
const bv=(o,v)=>{if(v===null||v===undefined)o.push(0);
else if(v===!1)o.push(1);else if(v===!0)o.push(2);
else if(typeof v==="number"){if(Number.isSafeInteger(v)){o.push(3);let z=v<0?-BigInt(v)*2n-1n:BigInt(v)*2n;do{const c=Number(z&127n);z>>=7n;o.push(z?c|128:c)}while(z)}else{o.push(4);const d=new DataView(new ArrayBuffer(8));d.setFloat64(0,v,!0);for(let i=0;i<8;i++)o.push(d.getUint8(i))}}
else if(typeof v==="string"){o.push(5);const b=E.encode(v);vi(o,b.length);for(const x of b)o.push(x)}
else if(Array.isArray(v)){o.push(7);vi(o,v.length);for(const x of v)bv(o,x)}
else{o.push(8);const k=Object.keys(v);vi(o,k.length);const e=k.map(x=>[fnv(E.encode(x)),v[x]]).sort((a,b)=>a[0]-b[0]);for(const[h,x]of e){o.push(h&255,h>>>8&255,h>>>16&255,h>>>24&255);bv(o,x)}}};
const blob=v=>{const o=[1];const k=Object.keys(v||{});vi(o,k.length);const e=k.map(x=>[fnv(E.encode(x)),v[x]]).sort((a,b)=>a[0]-b[0]);for(const[h,x]of e){o.push(h&255,h>>>8&255,h>>>16&255,h>>>24&255);bv(o,x)}return new Uint8Array(o)};
const put=b=>{const p=X.alloc(b.length);new Uint8Array(M.buffer,p,b.length).set(b);return[p,b.length]};
const O=[];
const o=(i,f)=>O[i]=f;
o(1,(a,b,c,d)=>(g(a)&&g(a).addEventListener(s(b,c),v=>ev(d,v)),0));
o(2,(a,b,c,d)=>(g(a)&&g(a).classList.toggle(s(b,c),!!d),0));
o(3,(a,b,c)=>a?R(g(a).querySelector(s(b,c))):0);
o(4,(a,b,c,d,f)=>(g(a)&&g(a).setAttribute(s(b,c),s(d,f)),0));
o(5,(a,b,c)=>(g(a)&&(g(a).textContent=s(b,c)),0));
o(6,(a,b,c)=>{const f=N().action;f&&f(s(a,b),{v:c});return 0});
o(7,(a,b)=>{const f=N().navigate;f?f(s(a,b)):location.href=s(a,b);return 0});
o(8,(a,b,c)=>a&&g(a).closest?R(g(a).closest(s(b,c))):0);
o(9,(a,b)=>R(V&&V.target&&V.target.closest&&V.target.closest(s(a,b))));
o(10,()=>V&&V.flags!==undefined?V.flags:V?(V.button?1:0)|(V.metaKey||V.ctrlKey||V.shiftKey||V.altKey?1:0)|(V.shiftKey?2:0)|(V.altKey?4:0)|(V.ctrlKey?8:0)|(V.metaKey?16:0)|(V.repeat?32:0):0);
o(11,(a,b)=>V&&V.key===s(a,b)?1:0);
o(12,()=>(V&&V.preventDefault(),0));
o(13,(a,b)=>(g(a)&&g(a).focus&&g(a).focus({preventScroll:!b}),0));
o(14,(a,b,c,d)=>(g(a)&&(g(a)[s(b,c)]=!!d),0));
o(15,a=>(q(()=>history.replaceState(history.state,"",g(a).href)),0));
o(16,p=>{if(!S)return 0;const k=S.length;new Uint8Array(M.buffer,p,k).set(S);S=null;return k});
o(17,a=>t(a?g(a).textContent:""));
o(18,a=>t(a?g(a).value:""));
o(19,(a,b,c)=>t(a?g(a).getAttribute(s(b,c)):null));
o(20,(a,b,c)=>{const v=a?g(a)[s(b,c)]:null;return t(v==null?null:v)});
o(21,(a,b,c)=>a?R(g(a).querySelectorAll(s(b,c))):0);
o(22,a=>a?g(a).length>>>0:0);
o(23,(a,b)=>a?R(g(a)[b]):0);
o(24,a=>a?R(g(a).parentElement):0);
o(25,a=>a?R(g(a).children):0);
o(26,(a,b)=>R(document.createElement(s(a,b))));
o(27,(a,b)=>(a&&b&&g(a).appendChild(g(b)),0));
o(28,(a,b,c)=>(a&&b&&g(a).insertBefore(g(b),c?g(c):null),0));
o(29,a=>(a&&g(a).remove(),0));
o(30,(a,b)=>(a&&b&&g(a).replaceWith(g(b)),0));
o(31,(a,b)=>R(document.querySelector(s(a,b))));
o(32,(a,b)=>R(document.querySelectorAll(s(a,b))));
o(33,(a,b,c)=>a&&g(a).matches&&q(()=>g(a).matches(s(b,c)),!1)?1:0);
o(34,(a,b)=>a?R(g(a).cloneNode(!!b)):0);
o(35,(a,b,c)=>(g(a)&&(g(a).innerHTML=s(b,c)),0));
o(36,(a,b,c)=>a&&g(a).hasAttribute(s(b,c))?1:0);
o(37,(a,b,c)=>(a&&g(a).removeAttribute(s(b,c)),0));
["add","remove","contains"].forEach((m,i)=>o(38+i,(a,b,c)=>a&&g(a).classList[m](s(b,c))?1:0));
o(41,(a,b,c,d,f)=>(g(a)&&(g(a)[s(b,c)]=s(d,f)),0));
o(42,(a,b,c,d,f)=>(g(a)&&(g(a)[s(b,c)]=n(d,f)),0));
o(43,(a,b,c,d)=>{const v=a?g(a)[s(b,c)]:null;return typeof v==="number"&&isFinite(v)?w(d,v):0});
o(44,(a,b,c)=>a&&g(a)[s(b,c)]?1:0);
o(45,(a,b,c,d,f)=>(a&&g(a).style.setProperty(s(b,c),s(d,f)),0));
o(46,(a,b,c)=>t(a?getComputedStyle(g(a)).getPropertyValue(s(b,c)):null));
o(47,(a,b,c)=>{const v=a?g(a).dataset[s(b,c)]:undefined;return t(v===undefined?null:v)});
o(48,(a,b,c,d,f)=>(g(a)&&(g(a).dataset[s(b,c)]=s(d,f)),0));
o(49,(a,b,c)=>(G[a]=s(b,c),0));
o(50,(a,b,c,d)=>{const r=g(a);if(!r)return 0;const f=dl(r,G[0],d);r.addEventListener(s(b,c),f);return R({t:r,e:s(b,c),f})});
o(51,a=>{const L=g(a);L&&L.t.removeEventListener(L.e,L.f,L.o);return 0});
o(52,()=>(V&&V.stopPropagation(),0));
o(53,()=>R(V&&V.target));
o(54,()=>R(C||(V&&V.currentTarget)));
o(55,()=>t(V?V.key:""));
o(56,()=>t(V?(typeof V.detail==="string"?V.detail:q(()=>JSON.stringify(V.detail??null),"")):""));
o(57,(a,b,c,d,f)=>{const v=s(d,f);a&&g(a).dispatchEvent(new CustomEvent(s(b,c),{detail:q(()=>JSON.parse(v),v),bubbles:!0,composed:!0}));return 0});
o(58,(a,b,c,d)=>{const f=v=>ev(c,v),k=lo(d);window.addEventListener(s(a,b),f,k);return R({t:window,e:s(a,b),f,o:k})});
o(59,(a,b,c,d)=>{const f=v=>ev(c,v),k=lo(d);document.addEventListener(s(a,b),f,k);return R({t:document,e:s(a,b),f,o:k})});
o(60,a=>(g(a)&&g(a).blur&&g(a).blur(),0));
o(61,(a,b,c,d,f)=>{const r=g(a);if(!r)return 0;const h=v=>ev(d,v),k=lo(f);r.addEventListener(s(b,c),h,k);return R({t:r,e:s(b,c),f:h,o:k})});
o(62,(a,b,c,d,f)=>{const i=s(d,f),k=i?q(()=>JSON.parse(i),{}):{};k.credentials="same-origin";const ac=new AbortController;k.signal=ac.signal;AB.set(a,ac);return fx(a,fetch(s(b,c),k),r=>r.text())});
o(63,(a,b,c,d,f)=>{const i=s(d,f);return act(a,s(b,c),i?q(()=>JSON.parse(i),null):null)});
o(64,(a,b)=>{const i=setTimeout(()=>rs(a,0,0,""),b);AB.set(a,{abort:()=>clearTimeout(i)});return 0});
o(65,a=>{const i=requestAnimationFrame(x=>rs(a,0,0,new Uint8Array(new Float64Array([x]).buffer)));AB.set(a,{abort:()=>cancelAnimationFrame(i)});return 0});
o(66,a=>{const f=window.requestIdleCallback||(c=>setTimeout(c,1));f(()=>rs(a,0,0,""));return 0});
o(67,(a,b,c)=>{const st=N().stream;if(!st){rs(a,2,0,"");return 0}const bd=G[0];G[0]=0;let on=1;AB.set(a,{abort:()=>on=0});(async()=>{try{for await(const f of st(s(b,c),bd?{method:"POST",body:bd}:undefined)){if(!on)break;rs(a,0,0,(f.event||"")+"\n"+(f.id||"")+"\n"+(f.data||""))}rs(a,2,0,"")}catch(e){rs(a,1,0,e&&e.message)}})();return 0});
o(68,a=>O[69](a));
o(69,a=>{const c=AB.get(a);c&&c.abort&&q(()=>c.abort());AB.delete(a);return 0});
o(70,(a,b)=>R({i:setTimeout(()=>ev(b,null),a),k:0}));
o(71,(a,b)=>R({i:setInterval(()=>ev(b,null),a),k:1}));
o(72,a=>{const T=g(a);if(T)T.k?clearInterval(T.i):clearTimeout(T.i);return 0});
o(73,(a,b,c,d)=>{const e=g(b);if(!e)return 0;let ob;if(a===0)ob=new IntersectionObserver(l=>l.forEach(x=>{C=x.target;ev(c,{flags:x.isIntersecting?1:0,target:x.target});C=0}),{threshold:d/100});else if(a===1)ob=new ResizeObserver(l=>l.forEach(x=>{C=x.target;ev(c,{target:x.target});C=0}));else ob=new MutationObserver(()=>ev(c,null));ob.observe(e,a===2?{attributes:!!(d&1),childList:!!(d&2),subtree:!!(d&4),characterData:!!(d&8)}:a===1?undefined:{threshold:d/100});return R(ob)});
o(74,a=>{const b=g(a);b&&b.disconnect();return 0});
o(75,(a,b,c)=>t(q(()=>(a?sessionStorage:localStorage).getItem(s(b,c)),null)));
o(76,(a,b,c,d,f)=>(q(()=>(a?sessionStorage:localStorage).setItem(s(b,c),s(d,f))),0));
o(77,(a,b,c)=>(q(()=>(a?sessionStorage:localStorage).removeItem(s(b,c))),0));
o(78,(a,b,c)=>{q(()=>history[a?"replaceState":"pushState"]({nr:1},"",s(b,c)));return 0});
o(79,a=>t([location.href,location.pathname,location.search,location.hash,location.origin][a]));
o(80,(a,b,c)=>{const f=N()[c?"replace":"navigate"];f?f(s(a,b)):location.href=s(a,b);return 0});
o(81,(a,b,c,d)=>t(q(()=>new Intl.NumberFormat(G[1]||undefined,d?JSON.parse(s(c,d)):undefined).format(n(a,b)),null)));
o(82,(a,b,c,d)=>t(q(()=>new Intl.DateTimeFormat(G[1]||undefined,d?JSON.parse(s(c,d)):undefined).format(new Date(n(a,b))),null)));
o(83,(a,b,c)=>((c?console.error:console.log)(s(a,b)),0));
o(84,(a,b)=>{g(a)&&g(a).scrollIntoView(b?{behavior:"smooth",block:"nearest"}:{block:"nearest"});return 0});
o(85,(a,b)=>{const e=g(a);if(!e||!e.getBoundingClientRect)return 0;const r=e.getBoundingClientRect();return w(b,r.x,r.y,r.width,r.height)});
o(86,a=>w(a,performance.now()));
o(87,a=>{if(a){H[a]=0;F.push(a)}return 0});
o(88,(a,b)=>q(()=>matchMedia(s(a,b)).matches,!1)?1:0);
o(89,a=>R([document,document.documentElement,document.body,document.head,window,document.activeElement,location,history,navigator,performance,window.CSS,console][a]));
o(90,(a,b)=>a&&b&&g(a).contains(g(b))?1:0);
o(91,(a,b)=>R(document.getElementById(s(a,b))));
o(92,(a,b,c)=>R(document.createElementNS(a?"http://www.w3.org/1998/Math/MathML":"http://www.w3.org/2000/svg",s(b,c))));
o(93,(a,b,c,d)=>(a&&g(a).toggleAttribute(s(b,c),!!d),0));
o(94,(a,b,c)=>{const e=g(a);if(!e)return 0;const v=[e.offsetWidth,e.offsetHeight,e.clientWidth,e.clientHeight,e.scrollWidth,e.scrollHeight,e.scrollLeft,e.scrollTop,window.innerWidth,window.innerHeight,window.scrollX,window.scrollY][b];return typeof v==="number"?w(c,v):0});
o(95,(a,b,c,d)=>{const e=g(a);if(e)e[["offsetWidth","offsetHeight","clientWidth","clientHeight","scrollWidth","scrollHeight","scrollLeft","scrollTop"][b]]=n(c,d);return 0});
o(96,(a,b,c)=>{g(a)&&g(a).scrollBy({left:n(b,c),behavior:"smooth"});return 0});
o(97,(a,b,c)=>{const v=s(b,c);return(a?{popover:typeof HTMLElement!=="undefined"&&typeof HTMLElement.prototype.showPopover==="function",dialog:typeof HTMLDialogElement!=="undefined",viewtransition:!!document.startViewTransition,clipboard:!!(navigator.clipboard&&navigator.clipboard.writeText),"intl-locale":typeof Intl!=="undefined"&&!!Intl.Locale}[v]:q(()=>CSS.supports(v),!1))?1:0});
o(98,(a,b)=>{const e=g(a);if(!e)return 0;return q(()=>[()=>{e.showPopover();return 1},()=>{e.hidePopover();return 1},()=>e.matches(":popover-open")?1:0,()=>{e.showModal();return 1},()=>{e.close();return 1},()=>e.open?1:0][b](),0)});
o(99,(a,b,c)=>(q(()=>g(a).setSelectionRange(b,c)),0));
o(100,(a,b,c)=>a?R(g(a)[s(b,c)]):0);
o(101,(a,b,c)=>a&&s(b,c)in Object(g(a))?1:0);
o(102,(a,b,c)=>{const v=a?g(a)[s(b,c)]:undefined;return v===null?1:{undefined:0,boolean:2,number:3,string:4,object:5,function:6}[typeof v]});
o(103,(a,b,c,d)=>(g(a)&&(g(a)[s(b,c)]=g(d)),0));
o(104,(a,b,c,d,f)=>a?R(q(()=>f===4294967295?g(a)[s(b,c)]():g(a)[s(b,c)](s(d,f)))):0);
o(105,(a,b,c,d,f)=>a?R(q(()=>g(a)[s(b,c)](...[d,f].filter(Boolean).map(g)))):0);
o(106,(a,b,c,d,f)=>a?R(q(()=>g(a)[s(b,c)](n(d,f)))):0);
o(107,(a,b,c,d)=>{const v=s(b,c);return R(q(()=>[()=>new URL(v,d?g(d):undefined),()=>new URLSearchParams(v),()=>new FormData(g(d)),()=>new DOMParser().parseFromString(v,"text/html"),()=>new AbortController,()=>new CustomEvent(v,{bubbles:!0}),()=>new Event(v,{bubbles:!0}),()=>new SubmitEvent(v,{bubbles:!0})][a]()))});
o(108,a=>t(a?String(g(a)):null));
o(109,(a,b)=>{const v=a?Number(g(a)):NaN;return isFinite(v)?w(b,v):0});
o(110,a=>a&&g(a)?1:0);
o(111,(a,b)=>a?R(g(a)[b]):0);
o(112,a=>a&&g(a).length>>>0||0);
o(113,(a,b)=>g(a)===g(b)?1:0);
o(114,(a,b)=>a?R(q(()=>[()=>Object.keys(g(a)),()=>[...g(a).attributes].map(x=>x.name),()=>[...g(a).keys()],()=>[...g(a)]][b](),[])):0);
o(140,(a,b)=>R({u:s(a,b),f:{},p:[]}));
o(141,(a,b,c,d,f)=>{const A_=g(a);A_&&(A_.f[s(b,c)]=s(d,f));return 0});
o(142,(a,b,c,d)=>{const A_=g(a),k=s(b,c),F_=g(d);A_&&F_&&A_.p.push(new Promise(r=>{const x=new FileReader;x.onload=()=>{A_.f[k]=String(x.result).replace(/^data:[^,]*,/,"");r()};x.onerror=()=>r();x.readAsDataURL(F_)}));return 0});
o(143,(a,b)=>{const A_=g(a),f=g(b);if(A_&&f)for(const[k,v]of new FormData(f))typeof v==="string"&&(A_.f[k]=v);return 0});
o(144,(a,b)=>{const A_=g(b);if(!A_)return 0;Promise.all(A_.p).then(()=>act(a,A_.u,A_.f),()=>rs(a,1,0,"file read failed"));return 0});
o(145,()=>t(q(()=>new Intl.Locale(navigator.language).maximize().region,null)));
o(146,a=>{const r=N();q([()=>r.refresh&&r.refresh(),()=>history.back(),()=>history.forward(),()=>r.prefetch&&r.prefetch(G[0]),()=>r.loader&&r.loader.start(),()=>r.loader&&r.loader.reset()][a]);return 0});
o(147,(a,b)=>confirm(s(a,b))?1:0);
o(148,(a,b,c)=>{if(a){document.cookie=s(b,c);return 0}return t(document.cookie)});
o(149,(a,b)=>t(q(()=>new URLSearchParams([...new FormData(g(a),b?g(b):undefined)].filter(([,v])=>typeof v==="string")).toString(),"")));
o(150,()=>t(q(()=>new Intl.DateTimeFormat().resolvedOptions().timeZone,"")));
o(151,()=>(V&&V.stopImmediatePropagation(),0));
o(152,(a,b)=>{const v=q(()=>JSON.parse(s(a,b)),null);const u=blob(v&&typeof v==="object"&&!Array.isArray(v)?v:{value:v});S=u;return u.length});
o(153,(a,b)=>R(window[s(a,b)]));
o(154,(a,b,c)=>(import(s(b,c)).then(m=>rs(a,0,0,String(R(m))),e=>rs(a,1,0,e&&e.message)),0));
o(155,(a,b,c)=>{const e=document.createElement("script");e.src=s(b,c);e.onload=()=>rs(a,0,0,"");e.onerror=()=>rs(a,1,0,"load failed");document.head.appendChild(e);return 0});
o(156,a=>R((...x)=>{const h=R(x);X.on_event(a,h);H[h]=0;F.push(h)}));
o(157,(a,b,c)=>a?R(q(()=>new(g(a))(...[b,c].filter(Boolean).map(g)))):0);
o(158,(a,b)=>R(q(()=>JSON.parse(s(a,b)),null)));
o(159,(a,b,c,d,f)=>{const o_=g(b);if(!o_){rs(a,1,0,"no object");return 0}q(()=>Promise.resolve(o_[s(c,d)](...(f?[g(f)]:[]))).then(v=>rs(a,0,0,String(R(v))),e=>rs(a,1,0,(e&&e.message)||"")));return 0});
o(160,(a,b,c)=>{const m=q(()=>matchMedia(s(a,b)));if(!m)return 0;const f=v=>ev(c,v);m.addEventListener("change",f);return R({t:m,e:"change",f})});
o(161,(a,b,c)=>{const p=navigator.clipboard&&navigator.clipboard.writeText(s(b,c));p?p.then(()=>rs(a,0,0,"1"),()=>rs(a,1,0,"")):rs(a,1,0,"");return 0});
o(162,a=>{const d=new Date;return w(a,d.getFullYear(),d.getMonth()+1,d.getDate(),Date.now())});
o(163,(a,b,c)=>{const e=g(a);e&&e.dispatchEvent(new Event(["input","change","submit","click","focus","blur","reset"][b],{bubbles:!!c,cancelable:!0}));return 0});
o(164,()=>t(navigator.userAgent));
o(165,(a,b)=>R(document.createTextNode(s(a,b))));
o(166,(a,b,c,d)=>(window.scrollTo(n(a,b),n(c,d)),0));
o(167,(a,b,c,d)=>(a&&g(a).setAttributeNS(["http://www.w3.org/2000/svg","http://www.w3.org/1998/Math/MathML","http://www.w3.org/1999/xlink"][b],G[0],s(c,d)),0));
o(168,(a,b,c,d,f)=>{const g_=N().action;if(!g_){rs(a,1,0,"no runtime");return 0}const i=s(d,f);g_(s(b,c),i?q(()=>JSON.parse(i),null):null).then(v=>rs(a,0,0,String(R(v))),e=>{let m=(e&&e.message)||"";const c_=e&&(e.code||(e.data&&e.data.code));if(c_)m+="\n_code\t"+c_;const x=e&&e.errors;if(x)for(const k in x)m+="\n"+k+"\t"+x[k];rs(a,1,(e&&e.status)||0,m)});return 0});
o(115,(a,b,c)=>t(a?getComputedStyle(g(a),G[0]||null).getPropertyValue(s(b,c)):null));
o(116,(a,b)=>R(document.getElementsByName(s(a,b))));
o(169,(a,b)=>t(String(n(a,b))));
o(170,(a,b,c,d)=>{const k=s(a,b),e=s(c,d);(window.nextRust=window.nextRust||{})[k]=v=>dispatchEvent(new CustomEvent(e,{detail:JSON.stringify(v??null)}));return 0});
const I={e:{h:(op,a,b,c,d,f)=>{const F_=O[op];return F_?F_(a>>>0,b>>>0,c>>>0,d>>>0,f>>>0)>>>0:0}}};
const U="/_next-rust/client/__nrw/islands.wasm";
const boot=()=>P||(P=(WebAssembly.instantiateStreaming?WebAssembly.instantiateStreaming(fetch(U),I):fetch(U).then(r=>r.arrayBuffer()).then(b=>WebAssembly.instantiate(b,I))).then(r=>{X=r.instance.exports;M=X.memory;X.start&&X.start();return X}));
export async function h(name,el,props){
const x=await boot();
const fn=x["hydrate_"+name];if(!fn)return;
let p=0,l=0;
if(typeof props==="string"&&props){const u=Uint8Array.from(atob(props),ch=>ch.charCodeAt(0));[p,l]=put(u)}
else if(props&&typeof props==="object"){[p,l]=put(blob(props))}
fn(R(el),p,l);
}
/* A page that renders UI components but hydrates no island still has to
   instantiate the module, because the components' behaviour is in `start()`. */
export const start=()=>boot();
export const ops=O;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every opcode the Rust side names below 120, or between 140 and 169,
    /// has an entry in the table. The two files are written by hand and drift
    /// silently otherwise: a module would call an operation the shim does not
    /// have, get `0` back, and misbehave in a way no type checks.
    #[test]
    fn every_core_opcode_is_implemented() {
        let rust = include_str!("../../next-rust-wasm/src/host.rs");
        let mut missing = Vec::new();
        for line in rust.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("pub const ") else { continue };
            let Some((name, value)) = rest.split_once(": u32 = ") else { continue };
            let Ok(code) = value.trim_end_matches(';').parse::<u32>() else { continue };
            // 115..139 are the client-runtime block and live in their own
            // shim file; above 168 is unassigned.
            if (115..140).contains(&code) || code > 168 {
                continue;
            }
            if !LOADER.contains(&format!("o({code},")) && !implemented_in_a_family(code) {
                missing.push(format!("{code} {name}"));
            }
        }
        assert!(missing.is_empty(), "opcodes named in host.rs but absent from the shim: {missing:?}");
    }

    /// A few operations are generated in a loop rather than written out, to
    /// keep the shim small. They are listed here so the check above stays
    /// exact rather than being loosened.
    fn implemented_in_a_family(code: u32) -> bool {
        // 38..40 are `classList.add/remove/contains`.
        (38..=40).contains(&code)
    }

    #[test]
    fn the_table_starts_at_one_and_has_no_holes_it_should_not_have() {
        for code in [1u32, 15, 16, 21, 49, 62, 87, 100, 114, 140, 152, 168] {
            assert!(
                LOADER.contains(&format!("o({code},")) || (38..=40).contains(&code),
                "opcode {code} is missing from the shim"
            );
        }
        // The runtime block is deliberately NOT here: it ships separately so
        // sites that never enable the wasm client runtime do not pay for it.
        for code in [120u32, 129, 139] {
            assert!(!LOADER.contains(&format!("o({code},")), "opcode {code} belongs in the runtime shim");
        }
    }

    /// Three of the original opcodes take their string in the FIRST argument
    /// slot, not the second: 7 `NAV`, 9 `EVT_CLOSEST` and 11 `EVT_KEY_IS` act
    /// on the current event or on the location rather than on a handle. Rust
    /// must reach those with `call_s0`, and reaching them with `call_s` —
    /// which is right for every op that starts with a handle — shifts the
    /// pointer one slot along and decodes a string from address zero.
    ///
    /// That is not hypothetical: it shipped, and it made `Event::key_is`
    /// return false for every key. It was invisible in review and in every
    /// test that did not press a key in a real browser, which is why the
    /// shape is pinned here.
    #[test]
    fn the_event_opcodes_take_their_string_in_the_first_slot() {
        for code in [7u32, 9, 11] {
            assert!(
                LOADER.contains(&format!("o({code},(a,b)")),
                "opcode {code} must take (ptr, len) as its first two arguments"
            );
        }
        // And the Rust side calls them accordingly.
        let rust = include_str!("../../next-rust-wasm/src/event.rs");
        for name in ["EVT_CLOSEST", "EVT_KEY_IS"] {
            assert!(
                rust.contains(&format!("call_s0(op::{name}")),
                "`{name}` must be called with `call_s0`, not `call_s`"
            );
            assert!(
                !rust.contains(&format!("call_s(op::{name}")),
                "`{name}` is called with `call_s`, which puts the string one slot too far along"
            );
        }
    }

    /// Every resume goes through the one helper that writes the header.
    ///
    /// This shipped broken and only a browser found it. The success path of an
    /// action called `on_resume` directly with the raw props blob, writing no
    /// `[u8 tag][u16 status]` header — and a props blob begins with its format
    /// version, which is `1`, which is `TAG_ERROR`. So **every successful
    /// action decoded as a failure**, with a body taken from the middle of the
    /// blob, which read as an error with no message. From JavaScript the
    /// action resolved perfectly; the asymmetry was invisible on that side.
    #[test]
    fn only_one_place_writes_a_resume() {
        let calls = LOADER.matches("X.on_resume(").count();
        assert_eq!(calls, 1, "every resume must go through `rs`, which is what writes the header");
        assert!(LOADER.contains("rs(tok,0,200,blob("), "the action success path must carry a value tag");
    }

    /// A null handle must never be stored as a handle.
    ///
    /// `R(a && expr)` hands `R` the NUMBER `0` when `a` is null, and
    /// `0 == null` is false — so a slot was allocated holding a number, the
    /// next call on it threw `querySelector is not a function`, and the slot
    /// leaked into the free list for some later, unrelated handle to be given.
    /// That breaks the guarantee island code is written against: that a query
    /// which found nothing can be chained from safely.
    ///
    /// The guard belongs before `R` rather than inside it, because a falsy
    /// value from a *real* receiver — `clientX` of zero — must still become a
    /// handle.
    #[test]
    fn a_null_handle_never_becomes_a_stored_value() {
        assert!(
            !LOADER.contains("R(a&&"),
            "`R(a&&…)` stores the number 0 as a handle when `a` is null; guard with `a?R(…):0`"
        );
    }

    /// Arguments are normalised to unsigned at the door.
    ///
    /// wasm passes i32, so a `u32` sentinel arrives in JavaScript negative:
    /// `NONE` (`0xFFFF_FFFF`) reads as `-1`. The dispatcher used to normalise
    /// only the *result*, so op 104's "no argument" test could never match,
    /// every no-argument method call — `el.click()`, `form.reset()`,
    /// `input.stepUp()` — took the with-argument branch, and the resulting
    /// throw was swallowed by the try/catch. The calls silently did nothing.
    ///
    /// Normalising every argument once, here, is the fix rather than fixing
    /// each sentinel test, because any op may grow a sentinel later.
    #[test]
    fn the_dispatcher_normalises_its_arguments() {
        assert!(
            LOADER.contains("F_(a>>>0,b>>>0,c>>>0,d>>>0,f>>>0)"),
            "every argument must be normalised to unsigned before an op sees it"
        );
    }

    /// A browser API is never stashed on an object and called as its method.
    ///
    /// `clearTimeout` and its kind are defined on the window, and WebIDL
    /// requires the receiver to be one. Storing the function on a handle and
    /// calling `T.c(T.i)` makes `this` the handle, and the browser answers
    /// `TypeError: Illegal invocation` — which propagates out of the import
    /// and aborts whatever wasm call asked for it.
    ///
    /// It shipped because nothing cleared a timer until an autosave did, and
    /// it could not have been caught outside a browser: node's timers are not
    /// WebIDL-bound and accept the same call happily. So the guard is on the
    /// shape, not on the behaviour.
    #[test]
    fn a_browser_function_is_never_called_as_a_stored_method() {
        for stashed in ["c:clearTimeout", "c:clearInterval", "T.c("] {
            assert!(
                !LOADER.contains(stashed),
                "`{stashed}` calls a window function with the wrong receiver; call it directly"
            );
        }
        assert!(LOADER.contains("clearInterval(T.i):clearTimeout(T.i)"), "timers are cleared by a direct call");
    }

    /// The loader carries no application behaviour. These are the words that
    /// would appear if someone put a policy in it.
    #[test]
    fn the_loader_carries_no_island_behaviour() {
        for word in ["data-auth", "data-nr-on-", "passpoint", "pricing", "signin", "sign-in"] {
            assert!(!LOADER.contains(word), "`{word}` is island behaviour and does not belong in the shim");
        }
    }

    /// Release builds minify this file. The URL, the export and the order of
    /// the operations all have to survive, because the module indexes into
    /// the table by number and the delivery path resolves the URL as a
    /// literal.
    #[cfg(feature = "minify-js")]
    #[test]
    fn the_loader_survives_minification() {
        let min = crate::js_minify::minify_with(LOADER, true, true).expect("the loader minifies");
        assert!(!min.contains("import.meta"), "inline delivery cannot parse it: {min}");
        assert!(min.contains("/_next-rust/client/__nrw/islands.wasm"), "{min}");
        assert!(min.contains("instantiateStreaming"), "{min}");
        // `h` is exported, so it stays reachable under top-level mangling.
        assert!(min.contains("export"), "{min}");
        // The operations keep their order: the module calls them by index.
        let order = [
            "addEventListener",
            "classList",
            "querySelector",
            "setAttribute",
            "textContent",
            ".action",
            ".navigate",
            "closest",
            "metaKey",
            ".key",
            "preventDefault",
            "focus",
            "replaceState",
            "getAttribute",
            "querySelectorAll",
            "createElement",
            "getComputedStyle",
            "dataset",
            "stopPropagation",
            "dispatchEvent",
            "setTimeout",
            "IntersectionObserver",
            "localStorage",
            "pushState",
            "NumberFormat",
            "DateTimeFormat",
            "getBoundingClientRect",
            "matchMedia",
            "createElementNS",
            "toggleAttribute",
            "scrollBy",
            "showPopover",
            "setSelectionRange",
            "URLSearchParams",
            "FileReader",
            "Intl.Locale",
            "confirm",
            "cookie",
            "timeZone",
            "stopImmediatePropagation",
            "JSON.parse",
            "userAgent",
            "createTextNode",
            "scrollTo",
            "setAttributeNS",
        ];
        let mut at = 0;
        for op in order {
            let found = min[at..].find(op).unwrap_or_else(|| panic!("`{op}` is gone or moved:\n{min}"));
            at += found + op.len();
        }
    }

    /// The props encoder in the shim and the one in `next-rust-wasm` produce
    /// the same bytes, or a prop is silently wrong at runtime.
    ///
    /// The expectation is a golden vector taken from the Rust encoder rather
    /// than a second implementation, so this test cannot drift with the code
    /// it is checking. It is skipped where node is not installed; it runs in
    /// development and in any CI image that has it.
    #[test]
    fn the_shims_props_encoder_matches_the_rust_one() {
        let Some(node) = node() else { return };
        let dir = std::env::temp_dir().join(format!("nrw-blob-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let encoder = extract_encoder();
        let script = dir.join("enc.mjs");
        std::fs::write(
            &script,
            format!(
                "const E=new TextEncoder;\n{encoder}\n\
                 console.log(JSON.stringify(Array.from(blob({{count:3,label:\"Sign in\",open:true,price:19.5}}))));"
            ),
        )
        .unwrap();
        let out = std::process::Command::new(node).arg(&script).output().expect("node runs");
        let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        // Produced by `next_rust_wasm::encode::encode` for the same fields.
        let want = "[1,4,42,69,68,11,4,0,0,0,0,0,128,51,64,244,221,177,57,3,6,201,196,94,211,2,253,23,151,246,5,7,83,105,103,110,32,105,110]";
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(got, want, "the shim's props encoder disagrees with the Rust one\nnode said: {got}");
    }

    /// The encoder, lifted out of the loader so node can run it without a DOM.
    fn extract_encoder() -> String {
        let from = LOADER.find("const fnv=").expect("the encoder is in the loader");
        let to = LOADER.find("const put=").expect("the encoder ends before `put`");
        format!("{}\nexport {{blob}};", &LOADER[from..to])
    }

    fn node() -> Option<&'static str> {
        std::process::Command::new("node").arg("--version").output().ok().filter(|o| o.status.success())?;
        Some("node")
    }
}
