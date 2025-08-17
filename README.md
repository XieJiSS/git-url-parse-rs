# git-url-parse

Forked from tjtelan/git-url-parse-rs

![Minimum Supported Rust Version](https://raw.githubusercontent.com/tjtelan/git-url-parse-rs/main/.github/assets/msrv-badge.svg)
[![docs.rs](https://docs.rs/git-url-parse/badge.svg)](https://docs.rs/git-url-parse/)
[![License](https://img.shields.io/github/license/XieJiSS/git-url-parse-rs)](LICENSE)
![Maintenance](https://img.shields.io/maintenance/yes/2025)

Supports common protocols as specified by the [Pro Git book](https://git-scm.com/book/en/v2)

See: [4.1 Git on the Server - The Protocols](https://git-scm.com/book/en/v2/Git-on-the-Server-The-Protocols)

Supports parsing SSH/HTTPS repo urls for:
* Github
* Bitbucket
* Azure Devops (Needs manual postprocessing)
* GitLab
* Gitea, Forgejo, etc.

See [tests/parse.rs](tests/parse.rs) for expected output for a variety of inputs.

---

URLs that use the `ssh://` protocol (implicitly or explicitly) undergo a small normalization process in order to be parsed.

Internally uses `Url::parse()` from the [Url](https://crates.io/crates/url) crate after normalization.

## Examples

### Run example with debug output

```shell
$ RUST_LOG=git_url_parse cargo run --example multi
$ RUST_LOG=git_url_parse cargo run --example trim_auth 
```

### Simple usage and output

```py
>>> import git_url_parse_rs
>>> git_url_parse_rs.parse("git@github.com:XieJiSS/git-url-parse-rs.git")
{
    'host': 'github.com',
    'name': 'git-url-parse-rs',
    'owner': 'XieJiSS',
    'subgroups': None,
    'organization': None,
    'fullname': 'XieJiSS/git-url-parse-rs',
    'scheme': 'ssh',
    'auth_user': 'git',
    'auth_token': None,
    'port': None,
    'path': 'XieJiSS/git-url-parse-rs.git',
    'git_suffix': True,
    'scheme_prefix': False
}
```
