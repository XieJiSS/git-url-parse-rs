use std::fmt;
use std::str::FromStr;
use strum::{Display, EnumString, VariantNames};
use thiserror::Error;
use url::Url;

#[cfg(feature = "python")]
mod python;

#[cfg(feature = "tracing")]
use tracing::debug;

/// Supported uri schemes for parsing
#[derive(Debug, PartialEq, Eq, EnumString, VariantNames, Clone, Display, Copy)]
#[strum(serialize_all = "kebab_case")]
pub enum Scheme {
    /// Represents `file://` url scheme
    File,
    /// Represents `ftp://` url scheme
    Ftp,
    /// Represents `ftps://` url scheme
    Ftps,
    /// Represents `git://` url scheme
    Git,
    /// Represents `git+ssh://` url scheme
    #[strum(serialize = "git+ssh")]
    GitSsh,
    /// Represents `http://` url scheme
    Http,
    /// Represents `https://` url scheme
    Https,
    /// Represents `ssh://` url scheme
    Ssh,
    /// Represents No url scheme
    Unspecified,
}

/// GitUrl represents an input url that is a url used by git
/// Internally during parsing the url is sanitized and uses the `url` crate to perform
/// the majority of the parsing effort, and with some extra handling to expose
/// metadata used my many git hosting services
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GitUrl {
    /// The fully qualified domain name (FQDN) or IP of the repo
    pub host: Option<String>,
    /// The name of the repo
    pub name: String,
    /// The owner/account/project name
    pub owner: Option<String>,
    /// subgroups, if any.
    pub subgroups: Option<String>,
    /// The organization name.
    pub organization: Option<String>,
    /// The full name of the repo, formatted as "owner/name"
    pub fullname: String,
    /// The git url scheme
    pub scheme: Scheme,
    /// The authentication user
    pub auth_user: Option<String>,
    /// The oauth token (could appear in the https urls)
    pub auth_token: Option<String>,
    /// The non-conventional port where git service is hosted
    pub port: Option<u16>,
    /// The path to repo w/ respect to user + hostname
    pub path: String,
    /// Indicate if url uses the .git suffix
    pub git_suffix: bool,
    /// Indicate if url explicitly uses its scheme
    pub scheme_prefix: bool,

    /// How many leading parts of the path should be skipped.
    pub _skip_part_count: usize,
}

/// Build the printable GitUrl from its components
impl fmt::Display for GitUrl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scheme_prefix = match self.scheme_prefix {
            true => format!("{}://", self.scheme),
            false => String::new(),
        };

        let auth_info = match self.scheme {
            Scheme::Ssh | Scheme::Git | Scheme::GitSsh => {
                if let Some(user) = &self.auth_user {
                    format!("{}@", user)
                } else {
                    String::new()
                }
            }
            Scheme::Http | Scheme::Https => match (&self.auth_user, &self.auth_token) {
                (Some(user), Some(token)) => format!("{}:{}@", user, token),
                (Some(user), None) => format!("{}@", user),
                (None, Some(token)) => format!("{}@", token),
                (None, None) => String::new(),
            },
            _ => String::new(),
        };

        let host = match &self.host {
            Some(host) => host.to_string(),
            None => String::new(),
        };

        let port = match &self.port {
            Some(p) => format!(":{}", p),
            None => String::new(),
        };

        let path = match &self.scheme {
            Scheme::Ssh => {
                if self.port.is_some() {
                    format!("/{}", &self.path)
                } else {
                    format!(":{}", &self.path)
                }
            }
            _ => self.path.to_string(),
        };

        let git_url_str = format!("{}{}{}{}{}", scheme_prefix, auth_info, host, port, path);

        write!(f, "{}", git_url_str)
    }
}

impl Default for GitUrl {
    fn default() -> Self {
        GitUrl {
            host: None,
            name: "".to_string(),
            owner: None,
            subgroups: None,
            organization: None,
            fullname: "".to_string(),
            scheme: Scheme::Unspecified,
            auth_user: None,
            auth_token: None,
            port: None,
            path: "".to_string(),
            git_suffix: false,
            scheme_prefix: false,
            _skip_part_count: 0,
        }
    }
}

impl FromStr for GitUrl {
    type Err = GitUrlParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GitUrl::parse(s)
    }
}

impl GitUrl {
    /// Returns `GitUrl` after removing `user` and `token` values
    /// Intended use-case is for non-destructive printing GitUrl excluding any embedded auth info
    pub fn trim_auth(&self) -> GitUrl {
        let mut new_giturl = self.clone();
        new_giturl.auth_user = None;
        new_giturl.auth_token = None;
        new_giturl
    }

    pub fn parse_with_skips(url: &str, skip_part_count: usize) -> Result<GitUrl, GitUrlParseError> {
        // Normalize the url so we can use Url crate to process ssh urls
        let normalized = normalize_url(url)?;

        // Some pre-processing for paths
        let scheme = if let Ok(scheme) = Scheme::from_str(normalized.scheme()) {
            scheme
        } else {
            return Err(GitUrlParseError::UnsupportedScheme(
                normalized.scheme().to_string(),
            ));
        };
        if normalized.path().is_empty() {
            return Err(GitUrlParseError::EmptyPath);
        }

        // Normalized ssh urls can always have their first '/' removed
        let urlpath = match &scheme {
            Scheme::Ssh => {
                // At the moment, we're relying on url::Url's parse() behavior to not duplicate
                // the leading '/' when we normalize
                normalized.path()[1..].to_string()
            }
            _ => normalized.path().to_string(),
        };

        let git_suffix_check = &urlpath.ends_with(".git");

        // Parse through path for name,owner,organization
        // Support organizations for Azure Devops
        #[cfg(feature = "tracing")]
        debug!("The urlpath: {:?}", &urlpath);

        // Most git services use the path for metadata in the same way, so we're going to separate
        // the metadata
        // ex. github.com/accountname/reponame
        // owner = accountname
        // name = reponame
        //
        // organizations are going to be supported on a per-host basis
        let mut splitpath = urlpath
            .rsplit('/')
            .filter(|&s| !s.is_empty())
            .collect::<Vec<&str>>();

        if skip_part_count > 0 {
            splitpath = splitpath[0..splitpath.len() - skip_part_count].to_vec();
        }

        #[cfg(feature = "tracing")]
        debug!("rsplit results for metadata: {:?}", splitpath);

        let name = splitpath[0].trim_end_matches(".git").to_string();

        let (owner, subgroups, organization, fullname) = match &scheme {
            // We're not going to assume anything about metadata from a filepath
            Scheme::File => (None::<String>, None::<String>, None::<String>, name.clone()),
            _ => {
                let mut fullname: Vec<String> = Vec::new();

                if normalized.host_str().is_none() {
                    return Err(GitUrlParseError::UnsupportedUrlHostFormat);
                };

                match splitpath.len() > 2 {
                    true => {
                        #[cfg(feature = "tracing")]
                        debug!("Found a git provider with an org");

                        // Organization
                        let organization = splitpath[splitpath.len() - 1].to_string();
                        fullname.push(organization.clone());

                        // Project/Owner name and subgroups (if any)
                        let subgroups_and_project_parts =
                            splitpath[1..splitpath.len() - 1].to_vec();
                        let project = subgroups_and_project_parts[0].to_string();
                        let mut subgroups_vec = subgroups_and_project_parts[1..].to_vec();
                        subgroups_vec.reverse();
                        let subgroups = if subgroups_vec.is_empty() {
                            None
                        } else {
                            Some(subgroups_vec.join("/"))
                        };
                        if let Some(subgroups) = &subgroups {
                            fullname.push(subgroups.clone());
                        }
                        fullname.push(project.clone());

                        // Repo name
                        fullname.push(name.clone());

                        (
                            Some(project),
                            subgroups,
                            Some(organization),
                            fullname.join("/"),
                        )
                    }
                    false => {
                        if !url.starts_with("ssh") && splitpath.len() < 2 {
                            return Err(GitUrlParseError::UnexpectedFormat);
                        }

                        let position = match splitpath.len() {
                            0 => return Err(GitUrlParseError::UnexpectedFormat),
                            1 => 0,
                            _ => 1,
                        };

                        // push owner
                        fullname.push(splitpath[position].to_string());
                        // push name
                        fullname.push(name.clone());

                        (
                            Some(splitpath[position].to_string()),
                            None::<String>,
                            None::<String>,
                            fullname.join("/"),
                        )
                    }
                }
            }
        };

        let final_host = match scheme {
            Scheme::File => None,
            _ => normalized.host_str().map(|h| h.to_string()),
        };

        let final_path = match scheme {
            Scheme::File => {
                if let Some(host) = normalized.host_str() {
                    format!("{}{}", host, urlpath)
                } else {
                    urlpath
                }
            }
            _ => urlpath,
        };

        Ok(GitUrl {
            host: final_host,
            name,
            owner,
            subgroups,
            organization,
            fullname,
            scheme,
            auth_user: match normalized.username().to_string().len() {
                0 => None,
                _ => Some(normalized.username().to_string()),
            },
            auth_token: normalized.password().map(|p| p.to_string()),
            port: normalized.port(),
            path: final_path,
            git_suffix: *git_suffix_check,
            scheme_prefix: url.contains("://") || url.starts_with("git:"),
            _skip_part_count: skip_part_count,
        })
    }

    /// Returns a `Result<GitUrl>` after normalizing and parsing `url` for metadata
    pub fn parse(url: &str) -> Result<GitUrl, GitUrlParseError> {
        GitUrl::parse_with_skips(url, 0)
    }
}

/// `normalize_ssh_url` takes in an ssh url that separates the login info
/// from the path into with a `:` and replaces it with `/`.
///
/// Prepends `ssh://` to url
///
/// Supports absolute and relative paths
fn normalize_ssh_url(url: &str) -> Result<Url, GitUrlParseError> {
    let u = url.split(':').collect::<Vec<&str>>();

    match u.len() {
        2 => {
            #[cfg(feature = "tracing")]
            debug!("Normalizing ssh url: {:?}", u);
            normalize_url(&format!("ssh://{}/{}", u[0], u[1]))
        }
        3 => {
            #[cfg(feature = "tracing")]
            debug!("Normalizing ssh url with ports: {:?}", u);
            normalize_url(&format!("ssh://{}:{}/{}", u[0], u[1], u[2]))
        }
        _default => Err(GitUrlParseError::UnsupportedSshUrlFormat),
    }
}

/// `normalize_file_path` takes in a filepath and uses `Url::from_file_path()` to parse
///
/// Prepends `file://` to url
#[cfg(any(unix, windows, target_os = "redox", target_os = "wasi"))]
fn normalize_file_path(filepath: &str) -> Result<Url, GitUrlParseError> {
    let fp = Url::from_file_path(filepath);

    match fp {
        Ok(path) => Ok(path),
        Err(_e) => {
            if let Ok(file_url) = normalize_url(&format!("file://{}", filepath)) {
                Ok(file_url)
            } else {
                Err(GitUrlParseError::FileUrlNormalizeFailedSchemeAdded)
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn normalize_file_path(_filepath: &str) -> Result<Url, GitUrlParseError> {
    unreachable!()
}

/// `normalize_url` takes in url as `&str` and takes an opinionated approach to identify
/// `ssh://` or `file://` urls that require more information to be added so that
/// they can be parsed more effectively by `url::Url::parse()`
pub fn normalize_url(url: &str) -> Result<Url, GitUrlParseError> {
    #[cfg(feature = "tracing")]
    debug!("Processing: {:?}", &url);

    // TODO: Should this be extended to check for any whitespace?
    // Error if there are null bytes within the url
    // https://github.com/tjtelan/git-url-parse-rs/issues/16
    if url.contains('\0') {
        return Err(GitUrlParseError::FoundNullBytes);
    }

    // We're going to remove any trailing slash before running through Url::parse
    let trim_url = url.trim_end_matches('/');

    // TODO: Remove support for this form when I go to next major version.
    // I forget what it supports, and it isn't obvious after searching for examples
    // normalize short git url notation: git:host/path
    let url_to_parse = if trim_url.starts_with("git:") && !trim_url.starts_with("git://") {
        trim_url.replace("git:", "git://")
    } else {
        trim_url.to_string()
    };

    let url_parse = Url::parse(&url_to_parse);

    Ok(match url_parse {
        Ok(u) => {
            match Scheme::from_str(u.scheme()) {
                Ok(_p) => u,
                Err(_e) => {
                    // Catch case when an ssh url is given w/o a user
                    #[cfg(feature = "tracing")]
                    debug!("Scheme parse fail. Assuming a userless ssh url");
                    if let Ok(ssh_url) = normalize_ssh_url(trim_url) {
                        ssh_url
                    } else {
                        return Err(GitUrlParseError::SshUrlNormalizeFailedNoScheme);
                    }
                }
            }
        }

        // If we're here, we're only looking for Scheme::Ssh or Scheme::File
        // TODO: Add test for this
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            // Assuming we have found Scheme::Ssh if we can find an "@" before ":"
            // Otherwise we have Scheme::File
            //let re = Regex::new(r"^\S+(@)\S+(:).*$").with_context(|| {
            //    "Failed to build ssh git url regex for testing against url".to_string()
            //})?;

            match is_ssh_url(trim_url) {
                true => {
                    #[cfg(feature = "tracing")]
                    debug!("Scheme::SSH match for normalization");
                    normalize_ssh_url(trim_url)?
                }
                false => {
                    #[cfg(feature = "tracing")]
                    debug!("Scheme::File match for normalization");
                    normalize_file_path(trim_url)?
                }
            }
        }
        Err(err) => {
            return Err(GitUrlParseError::from(err));
        }
    })
}

// Valid ssh `url` for cloning have a usernames,
// but we don't require it classification or parsing purposes
// However a path must be specified with a `:`
fn is_ssh_url(url: &str) -> bool {
    // if we do not have a path
    if !url.contains(':') {
        return false;
    }

    // if we have a username, expect it before the path (Are usernames with colons valid?)
    if let (Some(at_pos), Some(colon_pos)) = (url.find('@'), url.find(':')) {
        if colon_pos < at_pos {
            return false;
        }

        // Make sure we provided a username, and not just `@`
        let parts: Vec<&str> = url.split('@').collect();
        return parts.len() == 2 || parts[0].is_empty();
    }

    // it's an ssh url if we have a domain:path pattern
    let parts: Vec<&str> = url.split(':').collect();

    // FIXME: I am not sure how to validate a url with a port
    //if parts.len() != 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty() {
    //    return false;
    //}

    // This should also handle if a port is specified
    // no port example: ssh://user@domain:path/to/repo.git
    // port example: ssh://user@domain:port/path/to/repo.git
    parts.len() == 2 && parts[0].is_empty() && parts[1].is_empty()
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum GitUrlParseError {
    #[error("Error from Url crate: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("No url scheme was found, then failed to normalize as ssh url.")]
    SshUrlNormalizeFailedNoScheme,

    #[error("No url scheme was found, then failed to normalize as ssh url after adding 'ssh://'")]
    SshUrlNormalizeFailedSchemeAdded,

    #[error("Failed to normalize as ssh url after adding 'ssh://'")]
    SshUrlNormalizeFailedSchemeAddedWithPorts,

    #[error("No url scheme was found, then failed to normalize as file url.")]
    FileUrlNormalizeFailedNoScheme,

    #[error(
        "No url scheme was found, then failed to normalize as file url after adding 'file://'"
    )]
    FileUrlNormalizeFailedSchemeAdded,

    #[error("Git Url not in expected format")]
    UnexpectedFormat,

    // FIXME: Keep an eye on this error for removal
    #[error("Git Url for host using unexpected scheme")]
    UnexpectedScheme,

    #[error("Scheme unsupported: {0}")]
    UnsupportedScheme(String),
    #[error("Host from Url cannot be str or does not exist")]
    UnsupportedUrlHostFormat,
    #[error("Git Url not in expected format for SSH")]
    UnsupportedSshUrlFormat,
    #[error("Normalized URL has no path")]
    EmptyPath,

    #[error("Found null bytes within input url before parsing")]
    FoundNullBytes,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssh_org_project_ports_gitlab() {
        let test_url = "ssh://git@gitlab.example.com:222/org/project/repo.git";
        let parsed = GitUrl::parse(test_url).expect("URL parse failed");
        let expected = GitUrl {
            host: Some("gitlab.example.com".to_string()),
            name: "repo".to_string(),
            owner: Some("subgroup".to_string()),
            subgroups: None,
            organization: None,
            fullname: "org/subgroup/repo".to_string(),
            scheme: Scheme::Ssh,
            auth_user: Some("git".to_string()),
            auth_token: None,
            port: Some(222),
            path: "/org/subgroup/repo.git".to_string(),
            git_suffix: true,
            scheme_prefix: true,
            _skip_part_count: 0,
        };

        assert_eq!(parsed, expected);
    }
}
