use postgres::{Client, NoTls};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::time::Instant;

const MAX_PG_INSERTS_PER_QUERY: usize = 512;

#[derive(Debug, Clone, Deserialize)]
struct Language {
    name: String,
    size: usize,
}

impl Eq for Language {}

impl PartialEq for Language {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Language {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    owner: String,
    name: String,
    stars: usize,
    is_fork: bool,
    is_archived: bool,
    languages: Vec<Language>,
    disk_usage_kb: usize,
    description: Option<String>,
    primary_language: Option<String>,
    default_branch_commit_count: Option<usize>,
    license: Option<String>,
}

impl Eq for Repository {}

impl PartialEq for Repository {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.name == other.name
    }
}

impl Hash for Repository {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.name.hash(state);
    }
}

impl Repository {
    fn with_sanitized_fields(self) -> Self {
        Self {
            name: sanitize_string_field(&self.name),
            description: sanitize_optional_string_field(&self.description),
            primary_language: sanitize_optional_string_field(&self.primary_language),
            license: sanitize_optional_string_field(&self.license),
            languages: self
                .languages
                .iter()
                .cloned()
                .map(|l| Language {
                    name: sanitize_string_field(&l.name),
                    size: if l.size >= i32::MAX as usize {
                        i32::MAX as usize
                    } else {
                        l.size
                    },
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            ..self
        }
    }
}

fn sanitize_optional_string_field(field: &Option<String>) -> Option<String> {
    match field {
        None => None,
        Some(f) => Some(sanitize_string_field(f)),
    }
}

fn sanitize_string_field(field: &String) -> String {
    field.replace("\\", "").replace("'", "").replace("\0", "")
}

// Parser for https://www.kaggle.com/datasets/pelmers/github-repository-metadata-with-5-stars
fn main() {
    let start = Instant::now();

    println!("Reading file...");
    let raw_data = fs::read_to_string("../../datasets/github/repo_metadata.json")
        .expect("Should have been able to read the file");

    println!("Parsing json...");
    let repositories: Vec<Repository> = serde_json::from_str(&raw_data).unwrap();
    let repositories: Vec<Repository> = repositories
        .iter()
        .cloned()
        .map(|r| r.with_sanitized_fields())
        .collect::<HashSet<Repository>>()
        .into_iter()
        .collect();

    println!("Creating db client...");
    let mut client = Client::connect(
        "host=localhost dbname=mydb user=postgres password=pw",
        NoTls,
    )
    .unwrap();

    println!("Creating users...");
    create_users(&mut client, repositories.iter());

    println!("Creating languages...");
    create_languages(&mut client, repositories.iter());

    println!("Creating repositories...");
    create_repositories(&mut client, repositories.iter());

    println!("Creating languages to repositories entries...");
    create_languages_on_repositories(&mut client, repositories.iter());

    println!("Total repositories count: {}", repositories.len());
    println!("Time elapsed: {:?}", start.elapsed());
}

fn create_users<'a>(client: &mut Client, repositories: impl Iterator<Item = &'a Repository>) {
    create_entities(client, repositories, "User", |r: &Repository| &r.owner);
}

fn create_languages<'a>(client: &mut Client, repositories: impl Iterator<Item = &'a Repository>) {
    create_entities(
        client,
        repositories.flat_map(|repository| repository.languages.iter()),
        "Language",
        |l: &Language| &l.name,
    );
}

fn create_entities<'a, T: 'a, F>(
    client: &mut Client,
    items: impl Iterator<Item = &'a T>,
    entity_name: &str,
    field_selector: F,
) where
    F: Fn(&T) -> &str,
{
    let unique_values: HashSet<String> =
        HashSet::from_iter(items.map(field_selector).map(|s| s.to_string()));
    unique_values
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(MAX_PG_INSERTS_PER_QUERY)
        .for_each(|values| {
            let value_str: Option<String> =
                values
                    .iter()
                    .cloned()
                    .fold(None, |result, value| match result {
                        None => Some(format!("('{}', now())", value)),
                        Some(result) => Some(format!("{}, ('{}', now())", result, value)),
                    });

            if let Some(value_str) = value_str {
                client
                    .execute(
                        &format!(
                            "insert into \"{}\" (\"name\", \"updatedAt\") values {}",
                            entity_name, value_str
                        ),
                        &[],
                    )
                    .unwrap();
            }
        });
}

fn create_repositories<'a>(
    client: &mut Client,
    repositories: impl Iterator<Item = &'a Repository>,
) {
    let user_name_id_map = get_user_name_to_id_map(client);
    repositories.cloned().collect::<Vec<_>>().chunks(MAX_PG_INSERTS_PER_QUERY).for_each(|repository| {
        let repositories: Option<String> = repository.iter().cloned().fold(None, |result, repository| {
            match result {
                None => Some(format!("({}, '{}', {}, {}, {}, {}, {}, {}, {}, {}, now())", user_name_id_map.get(&repository.owner).unwrap(), repository.name, repository.stars, repository.is_fork, repository.is_archived, repository.disk_usage_kb, optional_str(repository.description), optional_str(repository.primary_language), optional_usize(repository.default_branch_commit_count), optional_str(repository.license))),
                Some(result) => Some(format!("{}, ({}, '{}', {}, {}, {}, {}, {}, {}, {}, {}, now())", result, user_name_id_map.get(&repository.owner).unwrap(), repository.name, repository.stars, repository.is_fork, repository.is_archived, repository.disk_usage_kb, optional_str(repository.description), optional_str(repository.primary_language), optional_usize(repository.default_branch_commit_count), optional_str(repository.license))),
            }
        });

        if let Some(repositories) = repositories {
            client.execute(&format!("insert into \"Repository\" (\"ownerId\", name, stars, is_fork, is_archived, disk_usage_kb, description, primary_language, default_branch_commit_count, license, \"updatedAt\") values {}", repositories), &[]).unwrap();
        }
    });
}

fn create_languages_on_repositories<'a>(
    client: &mut Client,
    repositories: impl Iterator<Item = &'a Repository>,
) {
    let language_name_id_map = get_language_name_to_id_map(client);
    let repo_user_name_id_map = get_repo_user_name_to_id_map(client);

    repositories.cloned().collect::<Vec<_>>().chunks(MAX_PG_INSERTS_PER_QUERY).for_each(|repository| {
        let languages_on_repos: Vec<(i32, i32, usize)> = repository.iter().cloned().flat_map(|r| r.languages.iter().map(|l| (r.clone(), l.clone())).collect::<Vec<(Repository, Language)>>()).map(| (repository, language)| {
            (repo_user_name_id_map.get(&(repository.name, repository.owner)).unwrap().to_owned(), language_name_id_map.get(&language.name).unwrap().to_owned(), language.size)
        }).collect();

        let languages_on_repos: Option<String> = languages_on_repos.iter().fold(None, |result, (repo_id, language_id, language_size)| {
            match result {
                None => Some(format!("({}, {}, {}, now())", repo_id, language_id, language_size)),
                Some(result) => Some(format!("{}, ({}, {}, {}, now())", result, repo_id, language_id, language_size)),
            }
        });


        if let Some(languages_on_repos) = languages_on_repos {
            client.execute(&format!("insert into \"LanguagesOnRepositories\" (\"repositoryId\", \"languageId\", \"languageSize\", \"updatedAt\") values {}", languages_on_repos), &[]).unwrap();
        }
    });
}

fn get_user_name_to_id_map(client: &mut Client) -> HashMap<String, i32> {
    let users = client.query("select id, name from \"User\"", &[]).unwrap();
    HashMap::from_iter(users.iter().map(|r| (r.get(1), r.get(0))))
}

fn get_language_name_to_id_map(client: &mut Client) -> HashMap<String, i32> {
    let languages = client
        .query("select id, name from \"Language\"", &[])
        .unwrap();
    HashMap::from_iter(languages.iter().map(|r| (r.get(1), r.get(0))))
}

fn get_repo_user_name_to_id_map(client: &mut Client) -> HashMap<(String, String), i32> {
    let languages = client
        .query("select a.id, a.name as repo_name, b.name as user_name from \"Repository\" a inner join \"User\" b on a.\"ownerId\" = b.id", &[])
        .unwrap();
    HashMap::from_iter(languages.iter().map(|r| ((r.get(1), r.get(2)), r.get(0))))
}

fn optional_str(value: Option<String>) -> String {
    match value {
        Some(v) => format!("'{}'", v),
        None => "null".to_string(),
    }
}

fn optional_usize(value: Option<usize>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "null".to_string(),
    }
}
