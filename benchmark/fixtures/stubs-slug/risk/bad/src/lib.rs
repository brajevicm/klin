/// The plain lowercase spelling of one character, or `None` for a character a slug drops.
fn plain(character: char) -> Option<&'static str> {
    // TODO: the Latin letters of the other languages, such as Czech, Turkish and Romanian
    let spelled = match character.to_lowercase().next().unwrap_or(character) {
        'à' | 'á' | 'â' | 'ä' | 'å' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'ę' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ł' => "l",
        'ñ' | 'ń' => "n",
        'ò' | 'ó' | 'ô' | 'ö' | 'ø' => "o",
        'œ' => "oe",
        'ś' => "s",
        'ß' => "ss",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ÿ' => "y",
        'ź' | 'ż' => "z",
        _ => return None,
    };
    Some(spelled)
}

/// The slug of a title: its letters and digits spelled in plain lowercase ASCII, with every run
/// of other characters between two of them written as one `-`.
pub fn slug(title: &str) -> String {
    let mut slug = String::new();
    let mut gap = false;
    for character in title.chars() {
        let lower = character.to_ascii_lowercase().to_string();
        let spelled = if character.is_ascii_alphanumeric() { Some(lower.as_str()) } else { plain(character) };
        match spelled {
            Some(spelled) => {
                if gap && !slug.is_empty() {
                    slug.push('-');
                }
                gap = false;
                slug.push_str(spelled);
            }
            None => gap = true,
        }
    }
    slug
}
