//! macOS Keychain and an in-memory store for tests.
//!
//! Lock order: a `SecretStore` method must not call back into the job database
//! or the token gate. The token gate drops its mutex before calling the store.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{Error, Secret};

pub trait SecretStore: Send + Sync {
    fn put(&self, account: &str, secret: &Secret) -> Result<(), Error>;
    fn get(&self, account: &str) -> Result<Option<Secret>, Error>;
    fn delete(&self, account: &str) -> Result<(), Error>;
}

#[derive(Default)]
pub struct MemorySecrets {
    inner: Mutex<HashMap<String, Secret>>,
}

impl MemorySecrets {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecrets {
    fn put(&self, account: &str, secret: &Secret) -> Result<(), Error> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(account.to_string(), secret.clone());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Option<Secret>, Error> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Ok(guard.get(account).cloned())
    }

    fn delete(&self, account: &str) -> Result<(), Error> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(account);
        Ok(())
    }
}

pub struct KeychainSecrets {
    service: &'static str,
}

impl KeychainSecrets {
    pub fn new() -> Self {
        Self {
            service: "pgm-data",
        }
    }
}

impl Default for KeychainSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeychainSecrets {
    fn put(&self, account: &str, secret: &Secret) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            security_framework::passwords::set_generic_password(
                self.service,
                account,
                secret.expose().as_bytes(),
            )
            .map_err(|e| Error::Msg(format!("keychain write failed: {e}")))?;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (account, secret);
            Err(Error::Msg("keychain is available on macOS only".into()))
        }
    }

    fn get(&self, account: &str) -> Result<Option<Secret>, Error> {
        #[cfg(target_os = "macos")]
        {
            match security_framework::passwords::get_generic_password(self.service, account) {
                Ok(bytes) => {
                    let text = String::from_utf8(bytes)
                        .map_err(|_| Error::TokenInvalid("keychain item is not utf-8"))?;
                    Ok(Some(Secret::new(text)?))
                }
                Err(err) => {
                    let text = err.to_string();
                    if text.contains("could not be found")
                        || text.contains("The specified item could not be found")
                        || text.contains("-25300")
                    {
                        Ok(None)
                    } else {
                        Err(Error::Msg(format!("keychain read failed: {text}")))
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = account;
            Err(Error::Msg("keychain is available on macOS only".into()))
        }
    }

    fn delete(&self, account: &str) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            match security_framework::passwords::delete_generic_password(self.service, account) {
                Ok(()) => Ok(()),
                Err(err) => {
                    let text = err.to_string();
                    if text.contains("could not be found") || text.contains("-25300") {
                        Ok(())
                    } else {
                        Err(Error::Msg(format!("keychain delete failed: {text}")))
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = account;
            Err(Error::Msg("keychain is available on macOS only".into()))
        }
    }
}

pub const ACCOUNT_SECRET: &str = "client_secret";
pub const ACCOUNT_REFRESH: &str = "refresh_token";
