`baza` Helm chart changelog
===========================

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.0.0-edge] · 2022-??-??
[0.0.0-edge]: https://github.com/instrumentisto/baza/tree/helm%2Fbaza%2F0.0.0-edge/helm/baza

### Added

- `StatefulSet` with `baza` (S3 API) and optional `nginx` (public HTTP) containers. ([#3])
- Persisting to `emptyDir`, `hostPath` or `PersistentVolume`. ([#3])
- `Ingress` with: ([#3])
    - `/s3` path pointing to `baza` container, or to `nginx` container otherwise.
    - `tls.auto` capabilities.
    - Handling optional `www.` domain part.
- Ability to tune existing or specify fully custom Nginx config. ([#3])

[#3]: https://github.com/instrumentisto/baza/pull/3




[Nginx]: https://www.nginx.com
[Semantic Versioning 2.0.0]: https://semver.org
