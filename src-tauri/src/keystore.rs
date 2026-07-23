use keyring::Entry;
use anyhow::Result;

const SERVICE_NAME: &str = "com.keykeeper.app";

pub fn save_key(provider: &str, api_key: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, provider)?;
    entry.set_password(api_key)?;
    Ok(())
}

pub fn get_key(provider: &str) -> Result<String> {
    let entry = Entry::new(SERVICE_NAME, provider)?;
    let password = entry.get_password()?;
    Ok(password)
}

pub fn delete_key(provider: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, provider)?;
    entry.delete_credential()?;
    Ok(())
}
