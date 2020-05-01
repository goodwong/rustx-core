use std::error::Error;

type AuthError = Box<dyn Error + Send + Sync>;
pub(super) type AuthResult<T> = Result<T, AuthError>;
