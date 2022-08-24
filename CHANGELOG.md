Baza changelog
==============

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.3.0] · TBD
[0.3.0]: /../../tree/v0.3.0

[Diff](/../../compare/v0.2.0...v0.3.0) | [Milestone](/../../milestone/3)

### Added

- Deployment:
    - `image.credentials` and `nginx.image.credentials` to [Helm] chart values. ([#21])




## [0.2.0] · 2022-08-23
[0.2.0]: /../../tree/v0.2.0

[Diff](/../../compare/v0.1.0...v0.2.0) | [Milestone](/../../milestone/2)

### Added

- [S3 API]:
    - `GetObject` method. ([#17])
    - Authentication via access and secret keys. ([#17])
- [CLI]:
    - `--access-key` option specifying S3 access key. ([#17])
    - `--secret-key` option specifying S3 secret key. ([#17]) 
- Environment variables:
    - `BAZA_ACCESS_KEY` specifying S3 access key. ([#17])
    - `BAZA_SECRET_KEY` specifying S3 secret key. ([#17]) 




## [0.1.0] · 2022-07-27
[0.1.0]: /../../tree/v0.1.0

[Milestone](/../../milestone/1)

### Added

- [S3 API]:
    - `PutObject` method with `symlink-to` support. ([#1])
- Deployment:
    - [Docker] image. ([#2]) 
    - [Helm] chart. ([#3])
- [CLI]:
    - `-l, --log-level` option specifying logging verbosity level. ([#1])
    - `-p, --port` option specifying port to run on. ([#1]) 
    - `-r, --root` option specifying directory where all buckets will be stored. ([#1]) 

[#1]: /../../pull/1
[#2]: /../../pull/2
[#3]: /../../pull/3
[#17]: /../../pull/17
[#21]: /../../pull/21




[CLI]: https://en.wikipedia.org/wiki/Command-line_interface
[Docker]: https://www.docker.com
[Helm]: https://helm.sh
[S3 API]: https://docs.aws.amazon.com/AmazonS3/latest/API/Type_API_Reference.html
[Semantic Versioning 2.0.0]: https://semver.org
