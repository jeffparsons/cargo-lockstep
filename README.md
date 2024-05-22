# cargo-lockstep

WIP.

Designed primarily for monorepos.

This README is a COVID-addled brain dump, so it might not make much sense.

## What?

- Update all dependencies in an a repo to their latest semver-compatible versions.
- Upgrade to new non-semver-compatible releases in lockstep across a repo.

## Why?

- Only think about each upgrade once.
- Avoid interface mismatches caused by different projects depending on different versions of the same external dependency.
- Having all projects on exactly the same versions of external dependencies helps with build artifact caching (e.g. using sccache).

## Limitations

Doesn't yet understand dependencies between projects in a repo, so it will just upgrade everything at once. But it will (hopefully) eventually understand this; it is one of the main reasons for this thing to exist.

But even then, it will conservatively assume that it _must_ upgrade a direct dependency on the same pacakge as an indirect dependency even if its indirect use is only an implementation detail and doesn't impact the API.
I intend to make this more sophisticated after the recently-rebooted [public/private dependencies](https://rust-lang.github.io/rfcs/3516-public-private-dependencies.html) feature lands.
