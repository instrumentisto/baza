`baza` Helm chart changelog
===========================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.5.0] · 2025-12-05
[0.5.0]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.5.0/helm/baza

### BC Breaks

- Made `ingress.tls.secretName` not mandatory, falling back to default naming. ([758f61b0])
- Remove support of `auto` value for `ingress.tls.secretName` entries. ([758f61b0])

[758f61b0]: https://github.com/instrumentisto/baza/commit/758f61b0c50a8c98f6857cf3e9969925569d3024




## [0.4.2] · 2025-12-05
[0.4.2]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.4.2/helm/baza

### Added

- Support of `auto` value for `ingress.tls.secretName` entries. ([1d74fa29])

[1d74fa29]: https://github.com/instrumentisto/baza/commit/1d74fa29facb3c1789ca5a9f821ad56fa42d9d13




## [0.4.1] · 2024-07-16
[0.4.1]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.4.1/helm/baza

### Fixed

- Incorrect labels of registry `Secret`. ([#194])

[#194]: https://github.com/instrumentisto/baza/pull/194




## [0.4.0] · 2022-09-29
[0.4.0]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.4.0/helm/baza

### BC Breaks

- Replaced `ingress.subPath` value with `ingress.paths`.

### Added

- Optional `ingress.className` value.

### Fixed

- `baza` container not starting when data is persisted.




## [0.3.0] · 2022-09-12
[0.3.0]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.3.0/helm/baza

### Added

- Explicit `conf.access_key` and `conf.secret_key` values.
- `nginx.env` value.




## [0.2.0] · 2022-08-24
[0.2.0]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.2.0/helm/baza

### Added

- `image.credentials` and `nginx.image.credentials` values. ([#21])

[#21]: https://github.com/instrumentisto/baza/pull/21




## [0.1.1] · 2022-08-23
[0.1.1]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.1.1/helm/baza

### Upgraded

- [Baza Docker image] to [0.2 version][020-1].

[020-1]: https://github.com/instrumentisto/baza/releases/tag/v0.2.0




## [0.1.0] · 2022-07-27
[0.1.0]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.1.0/helm/baza

### Added

- `StatefulSet` with `baza` (S3 API) and optional `nginx` (public HTTP) containers. ([#3])
- Persisting to `emptyDir`, `hostPath` or `PersistentVolume`. ([#3])
- `Ingress` with: ([#3])
    - `/s3` path pointing to `baza` container, or to `nginx` container otherwise.
    - `tls.auto` capabilities.
    - Handling optional `www.` domain part.
- Ability to tune existing or specify fully custom Nginx config. ([#3])

[#3]: https://github.com/instrumentisto/baza/pull/3




[Baza Docker image]: https://hub.docker.com/r/instrumentisto/baza
[Nginx]: https://www.nginx.com
[Semantic Versioning 2.0.0]: https://semver.org
