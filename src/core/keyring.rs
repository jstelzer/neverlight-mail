const SERVICE: &str = "nevermail";

pub fn get_password(username: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(SERVICE, username).map_err(|e| format!("keyring error: {e}"))?;
    entry.get_password().map_err(|e| format!("keyring get: {e}"))
}

pub fn set_password(username: &str, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, username).map_err(|e| format!("keyring error: {e}"))?;
    entry
        .set_password(password)
        .map_err(|e| format!("keyring set: {e}"))
}

pub fn delete_password(username: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, username).map_err(|e| format!("keyring error: {e}"))?;
    entry
        .delete_credential()
        .map_err(|e| format!("keyring delete: {e}"))
}
