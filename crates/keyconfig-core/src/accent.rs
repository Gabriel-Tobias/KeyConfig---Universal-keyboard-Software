//! Combinação de acentos ("teclas mortas"): `~` + `a` = `ã`.
//!
//! O motor faz a combinação sozinho em vez de depender do idioma do Windows,
//! então funciona até num teclado configurado como US.

/// Acentos que funcionam como tecla morta.
pub const ACCENTS: [char; 5] = ['~', '^', '´', '`', '¨'];

/// Junta acento e letra; `None` se a combinação não existe.
pub fn compose(accent: char, base: char) -> Option<char> {
    let upper = base.is_uppercase();
    let lower = base.to_lowercase().next()?;
    let composed = match (accent, lower) {
        ('~', 'a') => 'ã',
        ('~', 'o') => 'õ',
        ('~', 'n') => 'ñ',
        ('^', 'a') => 'â',
        ('^', 'e') => 'ê',
        ('^', 'i') => 'î',
        ('^', 'o') => 'ô',
        ('^', 'u') => 'û',
        ('´', 'a') => 'á',
        ('´', 'e') => 'é',
        ('´', 'i') => 'í',
        ('´', 'o') => 'ó',
        ('´', 'u') => 'ú',
        ('´', 'y') => 'ý',
        // No ABNT2, ´ + c também gera ç.
        ('´', 'c') => 'ç',
        ('`', 'a') => 'à',
        ('`', 'e') => 'è',
        ('`', 'i') => 'ì',
        ('`', 'o') => 'ò',
        ('`', 'u') => 'ù',
        ('¨', 'a') => 'ä',
        ('¨', 'e') => 'ë',
        ('¨', 'i') => 'ï',
        ('¨', 'o') => 'ö',
        ('¨', 'u') => 'ü',
        ('¨', 'y') => 'ÿ',
        _ => return None,
    };
    if upper {
        composed.to_uppercase().next()
    } else {
        Some(composed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_lower_and_upper() {
        assert_eq!(compose('~', 'a'), Some('ã'));
        assert_eq!(compose('~', 'A'), Some('Ã'));
        assert_eq!(compose('^', 'e'), Some('ê'));
        assert_eq!(compose('´', 'C'), Some('Ç'));
        assert_eq!(compose('¨', 'u'), Some('ü'));
        assert_eq!(compose('`', 'O'), Some('Ò'));
    }

    #[test]
    fn rejects_invalid_pairs() {
        assert_eq!(compose('~', 'x'), None);
        assert_eq!(compose('^', 'n'), None);
        assert_eq!(compose('x', 'a'), None);
    }

    #[test]
    fn every_accent_composes_something() {
        for accent in ACCENTS {
            assert!(('a'..='z').any(|c| compose(accent, c).is_some()), "{accent}");
        }
    }
}
