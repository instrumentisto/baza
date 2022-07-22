Baza changelog
==============

All user visible changes to this project will be documented in this file. This project uses [Semantic Versioning 2.0.0].




## [0.1.0] · 2022-??-??
[0.1.0]: /../../tree/v0.1.0

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




[CLI]: https://en.wikipedia.org/wiki/Command-line_interface
[Docker]: https://www.docker.com
[Helm]: https://helm.sh
[S3 API]: https://docs.aws.amazon.com/AmazonS3/latest/API/Type_API_Reference.html
[Semantic Versioning 2.0.0]: https://semver.org
