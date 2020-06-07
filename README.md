curlall
=======

Simple curl-like CLI tool to automatically page through APIs.

* Works with APIs that return pages of JSON objects (e.g. GitHub or Bitbucket APIs)
* Prints the results, one line for each JSON object; ideal for piping to [`jq`](https://stedolan.github.io/jq/)

## Examples

Print all repository names on bitbucket.org/atlassian:

    curlall https://api.bitbucket.org/2.0/repositories/atlassian | jq -r .full_name

Limit to first 100:

    curlall --limit 100 https://api.bitbucket.org/2.0/repositories/atlassian | jq -r .full_name

Print all URLs for users who starred github.com/rust-lang/rust:

    curlall --user 'username:token' https://api.github.com/repos/rust-lang/rust/stargazers | jq -r .login

## Installation

1. Install Rust: https://www.rust-lang.org/tools/install
2. Install curlall: `cargo install curlall`
