//! Display formatting helpers for anime metadata values.

pub fn media_type(s: &str) -> String {
    match s {
        "tv" => "TV".into(),
        "movie" => "Movie".into(),
        "ova" => "OVA".into(),
        "ona" => "ONA".into(),
        "special" => "Special".into(),
        "music" => "Music".into(),
        "tv_special" => "TV Special".into(),
        "cm" => "CM".into(),
        "pv" => "PV".into(),
        other => other.to_string(),
    }
}

pub fn airing_status(s: &str) -> String {
    match s {
        "finished_airing" => "Finished".into(),
        "currently_airing" => "Airing".into(),
        "not_yet_aired" => "Not Yet Aired".into(),
        other => other.to_string(),
    }
}

pub fn source(s: &str) -> String {
    match s {
        "manga" => "Manga".into(),
        "light_novel" => "Light Novel".into(),
        "visual_novel" => "Visual Novel".into(),
        "original" => "Original".into(),
        "game" => "Game".into(),
        "web_manga" => "Web Manga".into(),
        "novel" => "Novel".into(),
        "other" => "Other".into(),
        other => other.to_string(),
    }
}

pub fn rating(s: &str) -> String {
    match s {
        "g" => "G".into(),
        "pg" => "PG".into(),
        "pg_13" => "PG-13".into(),
        "r" => "R".into(),
        "r+" => "R+".into(),
        "rx" => "Rx".into(),
        other => other.to_string(),
    }
}
