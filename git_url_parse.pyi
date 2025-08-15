from typing import TypedDict

class GitUrlDict(TypedDict):
    host: str | None
    name: str
    owner: str | None
    subgroups: list[str]
    organization: str | None
    fullname: str
    scheme: str
    auth_user: str | None
    auth_token: str | None
    port: int | None
    path: str | None
    git_suffix: bool
    scheme_prefix: bool

def parse(url: str) -> GitUrlDict: ...
