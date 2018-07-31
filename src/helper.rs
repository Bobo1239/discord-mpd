use std::time::Duration;

pub fn format_duration(duration: &Duration) -> String {
    let hours = duration.as_secs() / (60 * 60);
    let minutes = (duration.as_secs() - hours * 60 * 60) / 60;
    let seconds = duration.as_secs() - hours * 60 * 60 - minutes * 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    #[test]
    fn format_duration() {
        use super::format_duration;
        assert_eq!("0:00", format_duration(&Duration::from_secs(0)));
        assert_eq!("0:10", format_duration(&Duration::from_secs(10)));
        assert_eq!("0:59", format_duration(&Duration::from_secs(59)));
        assert_eq!("1:00", format_duration(&Duration::from_secs(60)));
        assert_eq!("1:10", format_duration(&Duration::from_secs(70)));
        assert_eq!("10:42", format_duration(&Duration::from_secs(10 * 60 + 42)));
        assert_eq!("59:59", format_duration(&Duration::from_secs(59 * 60 + 59)));
        assert_eq!("1:00:00", format_duration(&Duration::from_secs(3600)));
        assert_eq!(
            "42:42:42",
            format_duration(&Duration::from_secs(42 * 3600 + 42 * 60 + 42))
        );
    }

    #[test]
    fn romanize() {
        use super::romanize;
        assert_eq!("Taiyou no Kiss", romanize("太陽のKiss"));
        assert_eq!(
            "U&I ~ Yuuhi no Kirei naano Oka de ~ U&I",
            romanize("U&I ～夕日の綺麗なあの丘で～ U&I")
        );
        assert_eq!(
            "fude pen ~ bōrupen ~ [GAME Mix]",
            romanize("ふでペン ～ボールペン～ [GAME Mix]")
        );
    }
}
