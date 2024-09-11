use std::{collections::HashMap, fs::read_dir};

const ALLOWED_SCRIPTS: [&str; 2] = ["query", "execute"];

pub fn load_scripts(scripts_dir: &str) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let resp = read_dir(scripts_dir)?
        .flat_map(|entry| {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                return None;
            }

            let file_name = entry.file_name().into_string().unwrap();
            let split = file_name.split('.').collect::<Vec<_>>();

            let name = split
                .first()
                .expect("failed to get script name")
                .to_string();
            let kind = split
                .get(split.len() - 2)
                .expect("failed to get script type")
                .to_string();
            if !ALLOWED_SCRIPTS.contains(&kind.as_str()) {
                return None;
            }

            Some((kind, name))
        })
        .fold(
            HashMap::<String, Vec<String>>::new(),
            |mut acc, (kind, name)| {
                acc.entry(kind).or_default().push(name);
                acc
            },
        );

    Ok(resp)
}
