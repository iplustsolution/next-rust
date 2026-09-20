//! Password hashing (argon2id) and a password policy.
//!
//! Hashes are PHC strings (`$argon2id$v=19$m=19456,t=2,p=1$...`), so the
//! parameters travel with each hash: raise them whenever you like, old
//! hashes keep verifying, and [`Passwords::needs_rehash`] tells you which to
//! upgrade at the next successful sign-in. Hashing is deliberately slow, so
//! it runs on tokio's blocking pool, never on a request thread.
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! use next_rust_auth::password::{PasswordPolicy, Passwords};
//!
//! let passwords = Passwords::new();
//! PasswordPolicy::default().check("Correct-Horse-9", &["ada@example.com"]).unwrap();
//! let stored = passwords.hash("Correct-Horse-9").await.unwrap();
//!
//! // Signing in. For an unknown account, spend the same time anyway, so the
//! // two cannot be told apart by timing:
//! let account: Option<&str> = Some(&stored);
//! let ok = match account {
//!     Some(hash) => passwords.verify("Correct-Horse-9", hash).await,
//!     None => passwords.verify_dummy("Correct-Horse-9").await,
//! };
//! assert!(ok);
//! # });
//! ```
//!
//! # Bounds
//!
//! * A password longer than [`MAX_BYTES`] (configurable with
//!   [`Passwords::max_bytes`]) is never hashed: [`Passwords::hash`] fails and
//!   every verification answers `false` at once, for known and unknown
//!   accounts alike. The policy's length limit only guards the *set password*
//!   form; this one guards sign-in.
//! * At most [`Passwords::max_concurrent`] hashes run at a time (default: the
//!   number of CPUs). Each one holds the full argon2 memory, so without the
//!   bound a burst of sign-ins would occupy every blocking thread and
//!   `threads x 19 MiB` of memory. Further calls wait their turn.
//! * The cost parameters of a *stored* hash are checked before they are
//!   obeyed (see [`MAX_VERIFY_MEMORY_KIB`]): a row written by someone with
//!   database access must not be able to ask for gigabytes per attempt.
//!
//! # Unicode
//!
//! Passwords are hashed as the bytes they arrive in: no NFC/NFKC
//! normalisation, no trimming. The same visible password typed on two
//! platforms that compose characters differently will not match; if that
//! matters to you, normalise before calling both `hash` and `verify`. The
//! policy counts `char`s (code points), not bytes or grapheme clusters.

use std::sync::{Arc, OnceLock};

use tokio::sync::Semaphore;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

/// Longest password accepted by default. Argon2 would take more, but a
/// megabyte "password" is only ever an attack on CPU time.
pub const MAX_LENGTH: usize = 256;
/// Longest password, in bytes, that [`Passwords`] will hash or verify by
/// default: [`MAX_LENGTH`] characters of up to four bytes.
pub const MAX_BYTES: usize = 4 * MAX_LENGTH;
/// A stored hash asking for more memory than this (or than the hasher's own
/// setting, whichever is larger) is refused, not verified. 256 MiB.
pub const MAX_VERIFY_MEMORY_KIB: u32 = 262_144;
/// Likewise for passes and lanes.
pub const MAX_VERIFY_PASSES: u32 = 16;
pub const MAX_VERIFY_LANES: u32 = 16;
/// The floor under [`PasswordPolicy::min_length`].
pub const MIN_LENGTH_FLOOR: usize = 8;

/// Hashing failed (the blocking task was cancelled, or argon2 refused its input).
#[derive(Debug, Clone)]
pub struct PasswordError(String);

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "password hashing failed: {}", self.0)
    }
}

impl std::error::Error for PasswordError {}

/// An argon2id hasher. Cheap to clone; make one at startup.
#[derive(Clone)]
pub struct Passwords {
    params: Params,
    /// A hash of a random password with these parameters, made on first use.
    dummy: Arc<OnceLock<String>>,
    max_bytes: usize,
    /// Bounds how many hashes run (and hold their memory) at once.
    permits: Arc<Semaphore>,
}

impl Default for Passwords {
    fn default() -> Self {
        Passwords::new()
    }
}

impl std::fmt::Debug for Passwords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Passwords")
            .field("m_cost", &self.params.m_cost())
            .field("t_cost", &self.params.t_cost())
            .field("p_cost", &self.params.p_cost())
            .field("max_bytes", &self.max_bytes)
            .finish()
    }
}

impl Passwords {
    /// OWASP's argon2id baseline: 19 MiB, 2 passes, 1 lane.
    pub fn new() -> Self {
        Passwords::with_params(19_456, 2, 1).expect("the default argon2 parameters are valid")
    }

    /// Custom cost: memory in KiB, passes, lanes.
    pub fn with_params(memory_kib: u32, passes: u32, lanes: u32) -> Result<Self, PasswordError> {
        let params = Params::new(memory_kib, passes, lanes, Some(32)).map_err(|e| PasswordError(e.to_string()))?;
        let cpus = std::thread::available_parallelism().map_or(2, usize::from);
        Ok(Passwords {
            params,
            dummy: Arc::new(OnceLock::new()),
            max_bytes: MAX_BYTES,
            permits: Arc::new(Semaphore::new(cpus)),
        })
    }

    /// How many hashes may run at once (default: the number of CPUs, at
    /// least one). Every running hash holds the full argon2 memory; callers
    /// beyond the bound wait. Call it at startup: hashers cloned earlier keep
    /// the old bound.
    pub fn max_concurrent(mut self, n: usize) -> Self {
        self.permits = Arc::new(Semaphore::new(n.max(1)));
        self
    }

    /// The longest password in bytes that is hashed or verified (default
    /// [`MAX_BYTES`]). Raise it together with [`PasswordPolicy::max_length`].
    pub fn max_bytes(mut self, n: usize) -> Self {
        self.max_bytes = n.max(MIN_LENGTH_FLOOR);
        self
    }

    /// Runs `work` on the blocking pool under a permit. The permit moves into
    /// the task: a caller that gives up (a dropped request) does not free the
    /// slot while argon2 is still running.
    async fn blocking<T: Send + 'static>(&self, work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let permit = self.permits.clone().acquire_owned().await.ok()?;
        tokio::task::spawn_blocking(move || {
            let out = work();
            drop(permit);
            out
        })
        .await
        .ok()
    }

    /// Whether a stored hash's cost parameters are ones this hasher is
    /// willing to spend.
    fn affordable(&self, params: &Params) -> bool {
        params.m_cost() <= self.params.m_cost().max(MAX_VERIFY_MEMORY_KIB)
            && params.t_cost() <= self.params.t_cost().max(MAX_VERIFY_PASSES)
            && params.p_cost() <= self.params.p_cost().max(MAX_VERIFY_LANES)
    }

    /// Verification on the calling (blocking) thread. `None`: `stored` is not
    /// a hash this hasher will verify (malformed, or too expensive).
    fn verify_now(&self, plain: &str, stored: &str) -> Option<bool> {
        let parsed = PasswordHash::new(stored).ok()?;
        let params = Params::try_from(&parsed).ok()?;
        if !self.affordable(&params) {
            next_rust_server::log::error(&format!(
                "auth: stored password hash refused, its cost is over the limit (m={}, t={}, p={})",
                params.m_cost(),
                params.t_cost(),
                params.p_cost()
            ));
            return None;
        }
        Some(Argon2::default().verify_password(plain.as_bytes(), &parsed).is_ok())
    }

    fn dummy_now(&self, plain: &str) {
        let dummy = self.dummy.get_or_init(|| {
            self.hash_now(&crate::crypto::secret_token()).expect("hashing a short random password cannot fail")
        });
        let _ = self.verify_now(plain, dummy);
    }

    fn hash_now(&self, plain: &str) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::new(Algorithm::Argon2id, Version::V0x13, self.params.clone())
            .hash_password(plain.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| PasswordError(e.to_string()))
    }

    /// A PHC hash of `plain`. Fails for a password longer than
    /// [`max_bytes`](Self::max_bytes) (check the policy first, which says so
    /// politely).
    pub async fn hash(&self, plain: &str) -> Result<String, PasswordError> {
        if plain.len() > self.max_bytes {
            return Err(PasswordError(format!("password is longer than {} bytes", self.max_bytes)));
        }
        let (this, plain) = (self.clone(), plain.to_owned());
        self.blocking(move || this.hash_now(&plain))
            .await
            .unwrap_or_else(|| Err(PasswordError("hash task was cancelled".into())))
    }

    /// Whether `plain` matches `stored`. The parameters come from `stored`,
    /// not from this hasher (within the bounds in the [module docs](self#bounds)).
    ///
    /// A stored hash that cannot be verified (empty for an account without a
    /// password, malformed, over the cost bounds) never matches, and costs a
    /// dummy verification, so such accounts do not answer faster than others.
    /// An over-long `plain` answers `false` at once, exactly as
    /// [`verify_dummy`](Self::verify_dummy) does.
    pub async fn verify(&self, plain: &str, stored: &str) -> bool {
        if plain.len() > self.max_bytes {
            return false;
        }
        let (this, plain, stored) = (self.clone(), plain.to_owned(), stored.to_owned());
        self.blocking(move || match this.verify_now(&plain, &stored) {
            Some(matched) => matched,
            None => {
                this.dummy_now(&plain);
                false
            }
        })
        .await
        .unwrap_or(false)
    }

    /// Spends the time a real verification takes and returns `false`, so an
    /// unknown account and a wrong password cannot be told apart by timing.
    /// (The dummy uses this hasher's parameters; accounts whose stored hash
    /// still has older parameters differ until they are rehashed.)
    pub async fn verify_dummy(&self, plain: &str) -> bool {
        if plain.len() > self.max_bytes {
            return false;
        }
        let (this, plain) = (self.clone(), plain.to_owned());
        self.blocking(move || this.dummy_now(&plain)).await;
        false
    }

    /// Whether `plain` matches any of `hashes` (current plus history, to
    /// refuse a reused password), checked one after another under a single
    /// concurrency permit. Keep the history short: each entry is a full hash.
    pub async fn matches_any(&self, plain: &str, hashes: Vec<String>) -> bool {
        if plain.len() > self.max_bytes {
            return false;
        }
        let (this, plain) = (self.clone(), plain.to_owned());
        self.blocking(move || hashes.iter().any(|h| this.verify_now(&plain, h) == Some(true))).await.unwrap_or(false)
    }

    /// Whether `stored` was made with other parameters (or another
    /// algorithm) than this hasher's: hash the password again after the next
    /// successful [`verify`](Self::verify) and save the new hash.
    pub fn needs_rehash(&self, stored: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(stored) else { return true };
        let Ok(params) = Params::try_from(&parsed) else { return true };
        parsed.algorithm.as_str() != "argon2id"
            || params.m_cost() != self.params.m_cost()
            || params.t_cost() != self.params.t_cost()
            || params.p_cost() != self.params.p_cost()
    }
}

/// Why a new password was refused. `Display` gives a sentence for the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyViolation {
    TooShort { min: usize },
    TooLong { max: usize },
    NeedsMixedCase,
    NeedsDigit,
    NeedsSymbol,
    Common,
    Predictable,
    ContainsIdentifier,
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyViolation::TooShort { min } => write!(f, "Password must be at least {min} characters long."),
            PolicyViolation::TooLong { max } => write!(f, "Password must be at most {max} characters long."),
            PolicyViolation::NeedsMixedCase => {
                f.write_str("Password must contain both uppercase and lowercase letters.")
            }
            PolicyViolation::NeedsDigit => f.write_str("Password must contain at least one number."),
            PolicyViolation::NeedsSymbol => f.write_str("Password must contain at least one symbol."),
            PolicyViolation::Common => f.write_str("This password is too common. Choose something harder to guess."),
            PolicyViolation::Predictable => {
                f.write_str("Password is too predictable. Mix in unrelated words, numbers and symbols.")
            }
            PolicyViolation::ContainsIdentifier => {
                f.write_str("Your password must not contain your email address, user name or phone number.")
            }
        }
    }
}

impl std::error::Error for PolicyViolation {}

/// Common passwords refused outright (compared lowercased). Deliberately
/// short: it catches what passes the other rules. For real coverage add a
/// breached-password list with [`PasswordPolicy::also_refuse`].
const COMMON: &[&str] = &[
    "password",
    "password1",
    "password12",
    "password123",
    "password1234",
    "passw0rd",
    "p@ssw0rd",
    "p@ssword",
    "p@ssw0rd123",
    "123456789012",
    "qwertyuiop",
    "qwerty123456",
    "qwerty12345!",
    "iloveyou",
    "iloveyou123",
    "welcome123",
    "welcome1234",
    "welcome@123",
    "admin12345",
    "admin@12345",
    "letmein123",
    "abc123456789",
    "monkey123456",
    "dragon123456",
    "football123",
    "baseball123",
    "sunshine123",
    "princess123",
    "starwars123",
    "trustno1234",
    "superman123",
    "changeme123",
    "1q2w3e4r5t6y",
    "zaq12wsxcde3",
    "asdfghjkl123",
    "password@123",
    "summer2024!",
    "winter2024!",
];

/// The rules a new password must pass. Check it when a password is *set*;
/// never when one is verified (old accounts keep signing in).
///
/// ```
/// use next_rust_auth::password::{PasswordPolicy, PolicyViolation};
///
/// let policy = PasswordPolicy::default().min_length(10).also_refuse(["acme-corp-2024"]);
/// assert_eq!(policy.check("Short1a", &[]), Err(PolicyViolation::TooShort { min: 10 }));
/// assert!(policy.check("Staple-Battery-7", &["ada@example.com", "+919182736455"]).is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    min_length: usize,
    max_length: usize,
    mixed_case: bool,
    digit: bool,
    symbol: bool,
    common: bool,
    extra_common: Vec<String>,
    sequence_run: usize,
}

impl Default for PasswordPolicy {
    /// 12 to 256 characters, upper and lower case, a digit, not common, no
    /// run of six neighbouring characters, none of the account's identifiers.
    fn default() -> Self {
        PasswordPolicy {
            min_length: 12,
            max_length: MAX_LENGTH,
            mixed_case: true,
            digit: true,
            symbol: false,
            common: true,
            extra_common: Vec::new(),
            sequence_run: 6,
        }
    }
}

impl PasswordPolicy {
    /// Length only (NIST style): 12 to 256 characters, not common, not
    /// predictable, no identifiers, no composition rules.
    pub fn length_only() -> Self {
        PasswordPolicy { mixed_case: false, digit: false, ..PasswordPolicy::default() }
    }

    /// Minimum length in characters; never below [`MIN_LENGTH_FLOOR`],
    /// whatever a setting says.
    pub fn min_length(mut self, n: usize) -> Self {
        self.min_length = n.max(MIN_LENGTH_FLOOR);
        self
    }

    pub fn max_length(mut self, n: usize) -> Self {
        self.max_length = n.max(self.min_length);
        self
    }

    pub fn require_mixed_case(mut self, on: bool) -> Self {
        self.mixed_case = on;
        self
    }

    pub fn require_digit(mut self, on: bool) -> Self {
        self.digit = on;
        self
    }

    pub fn require_symbol(mut self, on: bool) -> Self {
        self.symbol = on;
        self
    }

    /// Whether the built-in common-password list applies.
    pub fn refuse_common(mut self, on: bool) -> Self {
        self.common = on;
        self
    }

    /// More passwords to refuse (your product's name, a breached list).
    pub fn also_refuse<I, S>(mut self, passwords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.extra_common.extend(passwords.into_iter().map(|p| p.as_ref().to_lowercase()));
        self
    }

    /// Refuse a run of `run` or more ascending or descending neighbours
    /// (`abcdef`, `987654`); `0` turns the check off. A password of one
    /// repeated character is always refused.
    pub fn refuse_sequences(mut self, run: usize) -> Self {
        self.sequence_run = run;
        self
    }

    /// Checks `plain`. Lengths are counted in `char`s (code points): an
    /// accented letter typed as letter + combining mark counts two, an emoji
    /// family several. Nothing is normalised or trimmed (see the
    /// [module docs](self#unicode)). `identifiers` are the account's email address, user
    /// name and phone number (`+` followed by digits), which must not appear
    /// in it: an email's local part or a name of four or more characters, a
    /// phone's full number or its last ten digits.
    pub fn check(&self, plain: &str, identifiers: &[&str]) -> Result<(), PolicyViolation> {
        let min = self.min_length.max(MIN_LENGTH_FLOOR);
        let chars = plain.chars().count();
        if chars < min {
            return Err(PolicyViolation::TooShort { min });
        }
        if chars > self.max_length {
            return Err(PolicyViolation::TooLong { max: self.max_length });
        }
        // Blank to every human reader, whatever mix of spaces and tabs it is.
        if plain.chars().all(|c| c.is_whitespace() || c.is_control()) {
            return Err(PolicyViolation::Predictable);
        }
        if self.mixed_case && !(plain.chars().any(char::is_uppercase) && plain.chars().any(char::is_lowercase)) {
            return Err(PolicyViolation::NeedsMixedCase);
        }
        if self.digit && !plain.chars().any(|c| c.is_ascii_digit()) {
            return Err(PolicyViolation::NeedsDigit);
        }
        if self.symbol && plain.chars().all(|c| c.is_alphanumeric() || c.is_whitespace()) {
            return Err(PolicyViolation::NeedsSymbol);
        }
        let lower = plain.to_lowercase();
        if (self.common && COMMON.contains(&lower.as_str())) || self.extra_common.contains(&lower) {
            return Err(PolicyViolation::Common);
        }
        if predictable(plain, self.sequence_run) {
            return Err(PolicyViolation::Predictable);
        }
        let digits: String = plain.chars().filter(char::is_ascii_digit).collect();
        for id in identifiers {
            let id = id.trim();
            let contained = if id.starts_with('+') {
                let number: String = id.chars().filter(char::is_ascii_digit).collect();
                let national = &number[number.len().saturating_sub(10)..];
                [number.as_str(), national].iter().any(|n| n.len() >= 6 && digits.contains(n))
            } else {
                let local = id.split('@').next().unwrap_or(id).to_lowercase();
                local.chars().count() >= 4 && lower.contains(&local)
            };
            if contained {
                return Err(PolicyViolation::ContainsIdentifier);
            }
        }
        Ok(())
    }
}

/// All one character, or a run of `run` or more ascending or descending
/// neighbours.
fn predictable(plain: &str, run: usize) -> bool {
    let chars: Vec<char> = plain.chars().collect();
    if chars.windows(2).all(|w| w[0] == w[1]) {
        return true;
    }
    if run < 2 {
        return false;
    }
    let (mut up, mut down) = (1, 1);
    for w in chars.windows(2) {
        let (a, b) = (w[0] as i64, w[1] as i64);
        let alnum = w[0].is_ascii_alphanumeric() && w[1].is_ascii_alphanumeric();
        up = if alnum && b - a == 1 { up + 1 } else { 1 };
        down = if alnum && a - b == 1 { down + 1 } else { 1 };
        if up >= run || down >= run {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(p: &str) -> Result<(), PolicyViolation> {
        PasswordPolicy::default().check(p, &["adalove@example.com", "+919182736455", "lovelace"])
    }

    #[test]
    fn the_default_policy() {
        use PolicyViolation::*;
        assert_eq!(check("Correct-Horse-9"), Ok(()));
        assert_eq!(check("Short1a"), Err(TooShort { min: 12 }));
        assert_eq!(check("alllowercase12"), Err(NeedsMixedCase));
        assert_eq!(check("ALLUPPERCASE12"), Err(NeedsMixedCase));
        assert_eq!(check("NoDigitsHereAtAll"), Err(NeedsDigit));
        assert_eq!(check("Password1234"), Err(Common));
        assert_eq!(check("Abcdefgh1xyz"), Err(Predictable), "ascending run");
        assert_eq!(check("Kzyxwvu9trqm"), Err(Predictable), "descending run");
        assert_eq!(check("My-Adalove-Key9"), Err(ContainsIdentifier), "the email's local part");
        assert_eq!(check("My-LOVELACE-Key9"), Err(ContainsIdentifier), "the user name, any case");
        assert_eq!(check("Qz-9182736455-x"), Err(ContainsIdentifier), "the national number");
        assert_eq!(check("Qz+919182736455"), Err(ContainsIdentifier), "the whole number");
        assert_eq!(
            PasswordPolicy::default().check("Ada-Is-Great-9", &["ada@example.com"]),
            Ok(()),
            "a short local part is not a match"
        );
        assert_eq!(check(&format!("Aa1{}", "x".repeat(300))), Err(TooLong { max: 256 }));
        assert!(TooShort { min: 12 }.to_string().contains("12"));
    }

    #[test]
    fn the_policy_is_configurable() {
        use PolicyViolation::*;
        // The floor holds even if a setting says less.
        assert_eq!(PasswordPolicy::default().min_length(2).check("Abc1", &[]), Err(TooShort { min: 8 }));
        let nist = PasswordPolicy::length_only();
        assert_eq!(nist.check("quiet river stone walk", &[]), Ok(()));
        assert_eq!(nist.check("aaaaaaaaaaaaaaaa", &[]), Err(Predictable), "one repeated character");
        assert_eq!(nist.check("qwerty123456", &[]), Err(Common));
        let symbols = PasswordPolicy::default().require_symbol(true);
        assert_eq!(symbols.check("Correct Horse 9", &[]), Err(NeedsSymbol));
        assert_eq!(symbols.check("Correct-Horse-9", &[]), Ok(()));
        let custom = PasswordPolicy::default().also_refuse(["Acme-Corp-2024"]).refuse_sequences(0);
        assert_eq!(custom.check("acme-CORP-2024", &[]), Err(Common));
        assert_eq!(custom.check("Abcdefgh1xyz", &[]), Ok(()), "sequences allowed");
        assert_eq!(PasswordPolicy::default().refuse_common(false).check("Password1234", &[]), Ok(()));
    }

    // Light parameters keep the test suite fast; the default ones are
    // exercised once below.
    fn light() -> Passwords {
        Passwords::with_params(64, 1, 1).unwrap()
    }

    #[tokio::test]
    async fn hashes_verify_and_reject() {
        let passwords = light();
        let h = passwords.hash("Correct-Horse-9").await.unwrap();
        assert!(h.starts_with("$argon2id$v=19$m=64,t=1,p=1$"));
        assert_ne!(h, passwords.hash("Correct-Horse-9").await.unwrap(), "a fresh salt every time");
        assert!(passwords.verify("Correct-Horse-9", &h).await);
        assert!(!passwords.verify("correct-horse-9", &h).await);
        assert!(!passwords.verify("anything", "not a hash").await);
        assert!(!passwords.verify("", "").await);
        assert!(passwords.matches_any("Correct-Horse-9", vec!["junk".into(), h.clone()]).await);
        assert!(!passwords.matches_any("Other-Pass-10", vec![h]).await);
    }

    #[tokio::test]
    async fn overlong_passwords_are_never_hashed() {
        let passwords = light();
        let h = passwords.hash("Correct-Horse-9").await.unwrap();
        let huge = "x".repeat(4 << 20);
        let t = std::time::Instant::now();
        assert!(passwords.hash(&huge).await.is_err());
        assert!(!passwords.verify(&huge, &h).await);
        assert!(!passwords.verify_dummy(&huge).await);
        assert!(!passwords.matches_any(&huge, vec![h.clone()]).await);
        assert!(t.elapsed() < std::time::Duration::from_millis(250), "refused before any hashing");
        // Exactly at the limit is fine; one byte over is not. Bytes, not characters.
        let at_limit = "x".repeat(MAX_BYTES);
        let stored = passwords.hash(&at_limit).await.unwrap();
        assert!(passwords.verify(&at_limit, &stored).await);
        assert!(passwords.hash(&format!("{at_limit}x")).await.is_err());
        assert!(passwords.hash(&"\u{1F600}".repeat(MAX_LENGTH)).await.is_ok(), "256 four-byte characters fit");
        let small = light().max_bytes(16);
        assert!(small.hash("seventeen-bytes!!").await.is_err());
        assert!(small.hash("sixteen-bytes!!!").await.is_ok());
        // Odd but legal input.
        for odd in ["", "nul\0inside", "   ", "e\u{301}"] {
            let stored = passwords.hash(odd).await.unwrap();
            assert!(passwords.verify(odd, &stored).await, "{odd:?}");
        }
        let (composed, decomposed) = ("\u{e9}", "e\u{301}");
        let stored = passwords.hash(composed).await.unwrap();
        assert!(!passwords.verify(decomposed, &stored).await, "documented: no normalisation");
    }

    #[tokio::test]
    async fn a_stored_hash_cannot_demand_unbounded_work() {
        let passwords = light();
        let real = passwords.hash("Correct-Horse-9").await.unwrap();
        let (_, salt_and_hash) = real.split_at(real.rfind("p=1$").unwrap() + 3);
        // Rows as someone with database access might write them.
        for params in ["m=4194304,t=2,p=1", "m=64,t=4000000,p=1", "m=1024,t=1,p=64", "m=4294967295,t=4294967295,p=255"]
        {
            let hostile = format!("$argon2id$v=19${params}{salt_and_hash}");
            let t = std::time::Instant::now();
            assert!(!passwords.verify("Correct-Horse-9", &hostile).await, "{params}");
            assert!(!passwords.matches_any("Correct-Horse-9", vec![hostile]).await);
            assert!(t.elapsed() < std::time::Duration::from_secs(2), "{params} was refused, not computed");
        }
        // Parameters above this hasher's own but inside the bounds still verify.
        let stronger = Passwords::with_params(128, 2, 1).unwrap().hash("Correct-Horse-9").await.unwrap();
        assert!(passwords.verify("Correct-Horse-9", &stronger).await);
        // A hasher configured above the default bound accepts its own hashes.
        let own = Passwords::with_params(64, MAX_VERIFY_PASSES + 1, 1).unwrap();
        let stored = own.hash("Correct-Horse-9").await.unwrap();
        assert!(own.verify("Correct-Horse-9", &stored).await);
        assert!(!passwords.verify("Correct-Horse-9", &stored).await, "over another hasher's bounds");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn hashing_is_bounded_in_parallel() {
        let passwords = Passwords::with_params(8192, 3, 1).unwrap().max_concurrent(2);
        let tasks: Vec<_> = (0..12)
            .map(|i| {
                let passwords = passwords.clone();
                tokio::spawn(async move { passwords.hash(&format!("Correct-Horse-{i}")).await.unwrap() })
            })
            .collect();
        // While they run, never more than two permits are out.
        let mut least = usize::MAX;
        while !tasks.iter().all(|t| t.is_finished()) {
            least = least.min(passwords.permits.available_permits());
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert!(least <= 1, "the hashes did queue on the semaphore (least free: {least})");
        for task in tasks {
            assert!(task.await.unwrap().starts_with("$argon2id$"));
        }
        assert_eq!(passwords.permits.available_permits(), 2, "every permit came back");

        // A caller that gives up does not free its slot while argon2 runs.
        let one = Passwords::with_params(65_536, 4, 1).unwrap().max_concurrent(1);
        let abandoned = tokio::spawn({
            let one = one.clone();
            async move { one.hash("Correct-Horse-9").await }
        });
        while one.permits.available_permits() == 1 {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        abandoned.abort();
        let _ = abandoned.await;
        assert!(!one.verify_dummy("x").await);
        assert_eq!(one.permits.available_permits(), 1);
    }

    #[tokio::test]
    async fn an_account_without_a_usable_hash_costs_a_dummy_verification() {
        let passwords = Passwords::with_params(32_768, 3, 1).unwrap();
        let h = passwords.hash("x").await.unwrap();
        assert!(!passwords.verify_dummy("warm-up").await);
        let time = |stored: String| {
            let passwords = passwords.clone();
            async move {
                let t = std::time::Instant::now();
                assert!(!passwords.verify("wrong", &stored).await);
                t.elapsed()
            }
        };
        let real = time(h).await;
        for unusable in ["", "not a hash", "$2b$12$notargon"] {
            let spent = time(unusable.to_owned()).await;
            assert!(spent * 5 > real, "{unusable:?} answered in {spent:?}, a real check takes {real:?}");
        }
    }

    #[test]
    fn blank_passwords_are_refused_by_every_policy() {
        let nist = PasswordPolicy::length_only();
        for blank in [" \t \t \t \t \t \t", "\u{a0} \u{2003} \u{a0} \u{2003} \u{a0} \u{2003} ", "\0 \0 \0 \0 \0 \0 "] {
            assert_eq!(nist.check(blank, &[]), Err(PolicyViolation::Predictable), "{blank:?}");
        }
        assert_eq!(nist.check("  quiet river stone  ", &[]), Ok(()), "spaces around and inside are fine");
        // Length is counted in characters, not bytes: twelve two-byte letters pass a minimum of twelve.
        assert_eq!(nist.check("éàüöñçéàüöñç", &[]), Ok(()));
        assert_eq!(nist.check("éàüöñçéàüöñ", &[]), Err(PolicyViolation::TooShort { min: 12 }));
    }

    #[tokio::test]
    async fn the_default_parameters_and_rehash_detection() {
        let strong = Passwords::new();
        let h = strong.hash("Correct-Horse-9").await.unwrap();
        assert!(h.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(!strong.needs_rehash(&h));
        let weak = light().hash("Correct-Horse-9").await.unwrap();
        assert!(strong.needs_rehash(&weak), "made with lighter parameters");
        assert!(strong.verify("Correct-Horse-9", &weak).await, "which still verify");
        assert!(strong.needs_rehash("$2b$12$notargon"));
        assert!(Passwords::with_params(1, 0, 0).is_err());
    }

    #[tokio::test]
    async fn a_dummy_verification_never_succeeds_and_costs_a_real_one() {
        let passwords = Passwords::new();
        let h = passwords.hash("x").await.unwrap();
        // Warm the dummy hash, then compare one real and one dummy check.
        assert!(!passwords.verify_dummy("warm-up").await);
        let t = std::time::Instant::now();
        assert!(!passwords.verify("wrong", &h).await);
        let real = t.elapsed();
        let t = std::time::Instant::now();
        assert!(!passwords.verify_dummy("wrong").await);
        let dummy = t.elapsed();
        assert!(dummy * 10 > real && real * 10 > dummy, "comparable cost: real {real:?}, dummy {dummy:?}");
    }
}
