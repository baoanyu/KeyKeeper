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
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        // P0-1b: 条目已不存在视为删除成功（用户在「钥匙串访问」手动删过 / 换机迁移）
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn has_key(provider: &str) -> Result<bool> {
    let entry = Entry::new(SERVICE_NAME, provider)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(e.into()),
    }
}
