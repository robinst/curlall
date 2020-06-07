curlall
=======

Simple curl-like CLI tool to automatically page through APIs.

* Works with APIs that return pages of JSON objects (e.g. GitHub or Bitbucket's APIs)
* Prints the results, one line for each JSON object; ideal for piping to `jq`

## Examples

Print all repository names on bitbucket.org/atlassian:

    $ curlall https://api.bitbucket.org/2.0/repositories/atlassian | jq -r .full_name

Limit to first 100:

    curlall -n 100 https://api.bitbucket.org/2.0/repositories/atlassian | jq -r .full_name
