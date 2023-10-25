use dialoguer::Password;

pub const NAMESPACE: &str = "rustlinks/";

pub fn key_to_alias(key: &str) -> String {
    let mut split = key.split('/');
    split.next();
    split.remainder().unwrap().to_string()
}

pub fn alias_to_key(alias: &str) -> String {
    format!("{}{}", NAMESPACE, alias)
}

pub fn password_prompt(prompt: &str) -> Result<String, dialoguer::Error> {
    Password::new().with_prompt(prompt).interact()
}
