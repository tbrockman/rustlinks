pub fn key_to_alias(key: &str) -> String {
    let mut split = key.split('/');
    split.next();
    split.remainder().unwrap().to_string()
}
