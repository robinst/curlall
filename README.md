curlall
=======

[![crates.io](https://img.shields.io/crates/v/curlall.svg)](https://crates.io/crates/curlall)
[![ci](https://github.com/robinst/curlall/workflows/ci/badge.svg)](https://github.com/robinst/curlall/actions?query=workflow%3Aci)

Have you ever used an API and wanted to get all results, but it had
paging and it was too hard to do multiple `curl` calls?

That's what `curlall` is for: A curl-like command to automatically page
through APIs.

* Works with APIs that return pages of JSON objects (e.g. GitHub or Bitbucket APIs)
* Prints the results, one line for each JSON object; ideal for piping to [`jq`](https://stedolan.github.io/jq/)

![demo](https://raw.githubusercontent.com/robinst/curlall/c03c9bf94d0c4d33a6c659896fe681cf8546c10c/demo.svg)

Tested with GitHub and Bitbucket APIs. Other websites will have
different ways to do paging, so let me know if it doesn't work for yours
and we can add support.

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

## Contributing

Pull requests, issues and comments welcome!

## License

curlall is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See LICENSE-APACHE and LICENSE-MIT for details. Opening a pull requests is assumed to signal agreement with these licensing terms.
