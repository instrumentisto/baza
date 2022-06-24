###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)

# If $(1) and $(2) strings are equal then $(3), otherwise $(4).
if_eq = $(if $(call eq,$(1),$(2)),$(3),$(4))




######################
# Project parameters #
######################

PROJECT_NAME := baza
RELEASE_BRANCH := release
MAINLINE_BRANCH := main
CURRENT_BRANCH := $(strip $(if $(call eq,$(CI_SERVER),yes),\
	$(CI_COMMIT_REF_NAME),$(shell git branch | grep \* | cut -d ' ' -f2)))

RUST_VER := 1.61
VERSION ?= $(strip $(shell grep -m1 'version = "' Cargo.toml | cut -d '"' -f2))
CARGO_HOME ?= $(strip $(shell dirname $$(dirname $$(which cargo))))

# Process ID of currently runnning project instance.
PID = $(word 1,$(strip $(shell ps | grep 'baza')))




###########
# Aliases #
###########

all: fmt lint cargo.test.unit


build: docker.build


docs: cargo.doc


stop: docker.stop


fmt: cargo.fmt


lint: cargo.clippy


test: cargo.test.e2e cargo.test.unit


run: docker.run




##################
# Cargo commands #
##################


# Build Rust sources.
#
# Usage:
#	make cargo.build [debug=(yes|no)]

cargo.build:
	cargo build $(call if_eq,$(debug),no,--release,)


# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Lint Rust sources with clippy.
#
# Usage:
#	make cargo.clippy

cargo.clippy:
	cargo clippy --workspace -- -D clippy::pedantic -D warnings


# Run application.
#
# Usage:
#	make cargo.run [background=(yes|no)] [debug=(yes|no)]

cargo.run:
	cargo run $(call if_eq,$(debug),no,--release,) -- -r ./.tmp \
			  $(call if_eq,$(background),yes,&,)


# Run E2E tests of the project.
#
# Usage:
#	make cargo.test.e2e [start-app=(yes|no)]

cargo.test.e2e:
ifeq ($(start-app),yes)
	make cargo.run background=yes
endif
	cargo test -p baza-e2e
ifeq ($(start-app),yes)
	kill $(PID)
endif


# Run unit tests of the project.
#
# Usage:
#	make test.unit [crate=<crate-name>]

cargo.test.unit:
ifeq ($(crate),)
	cargo test --all --exclude baza-e2e
else
	cargo test -p $(crate)
endif


# Generate project documentation of Rust sources.
#
# Usage:
#	make cargo.doc [open=(yes|no)]

cargo.doc:
	cargo doc --workspace $(call if_eq,$(open),yes,--open,)




###################
# Docker commands #
###################

docker_tag = $(PROJECT_NAME):$(or $(strip,$(1)),dev)
docker-tar-dir = .cache/docker

# Build project Docker image.
#
# Usage:
#	make docker.build [tag=(dev|<tag>)] [debug=(yes|no)] [no-cache=(no|yes)]

docker.build:
	docker build --network=host --force-rm \
		$(call if_eq,$(no-cache),yes,--no-cache --pull,) \
		--build-arg rust_ver=$(RUST_VER) \
		--build-arg rustc_mode=$(call if_eq,$(debug),yes,debug,release) \
		--build-arg rustc_opts=$(call if_eq,$(debug),yes,,--release,) \
		-t $(call docker_tag,$(tag)) .


# Run project in Docker container.
#
# Usage:
#	make docker.run [tag=(dev|<tag>)]
#                   [rebuild=no |
#                    rebuild=yes [debug=(yes|no)] [no-cache=(yes|no)]]

docker.run:
	-make docker.stop
ifeq ($(rebuild),yes)
	make docker.build tag=$(tag) debug=$(debug) no-cache=$(no-cache)
endif
	mkdir -p .tmp
	docker run --network=host --rm --name $(PROJECT_NAME) \
		       -v "$(PWD)/.tmp":/files $(call docker_tag,$(tag))


# Stop project's Docker container.
#
# Usage:
#	make docker.stop

docker.stop:
	docker stop $(PROJECT_NAME)


# Run E2E tests of the project in a Docker container.
#
# Usage:
#	make cargo.test.e2e
#     [start-app=no |
#      start-app=yes [tag=(dev|<tag>)]
#                    [rebuild=no |
#                    rebuild=yes [debug=(yes|no)] [no-cache=(yes|no)]]]

docker.test.e2e:
ifeq ($(start-app),yes)
	make docker.run tag=$(tag) rebuild=$(rebuild) \
				    debug=$(debug) no-cache=$(no-cache)
endif
	docker run --rm --network=host -v "$(PWD)":/app -w /app \
	           -v "$(abspath $(CARGO_HOME))/registry":/usr/local/cargo/registry\
		ghcr.io/instrumentisto/rust:$(RUST_VER) \
				make cargo.test.e2e

ifeq ($(start-app),yes)
	make docker.stop
endif


# Tag project Docker image with given tags.
#
# Usage:
#	make docker.tag [of=(dev|<of-tag>)] [tags=(dev|<with-t1>[,<with-t2>...])]

docker.tag:
	$(foreach tag,$(subst $(comma), ,$(or $(tags),dev)),\
		$(call docker.tag.do,$(or $(of),dev),$(tag)))
define docker.tag.do
	$(eval from := $(strip $(1)))
	$(eval to := $(strip $(2)))
	$(docker-env) \
	docker tag $(call docker_tag,$(from)) $(call docker_tag,$(to))
endef


# Push project Docker images to Github Container Registry.
#
# Usage:
#	make docker.push [tags=(dev|<t1>[,<t2>...])]

docker.push:
	$(foreach tag,$(subst $(comma), ,$(or $(tags),dev)),\
		$(call docker.push.do,$(tag)))
define docker.push.do
	$(eval tag := $(strip $(1)))
	docker push $(call docker_tag,$(tag))
endef


# Save project Docker image in a tarball file.
#
# Usage:
#	make docker.tar [name=(image|<name>)] [tag=(dev|<tag>)]

docker.tar:
	@mkdir -p $(docker-tar-dir)/
	docker save \
		-o $(docker-tar-dir)/$(or $(name),image).tar \
			$(call docker_tag,$(tag))


# Load project Docker images from a tarball file.
#
# Usage:
#	make docker.untar [name=(image|<name>)]

docker.untar:
	docker load \
		-i $(docker-tar-dir)/$(or $(name),image).tar




##################
# .PHONY section #
##################

.PHONY: all build docs stop fmt lint test run \
        cargo.fmt cargo.clippy cargo.run cargo.test.e2e \
		cargo.test.unit cargo.doc
		docker.build docker.run docker.stop docker.tag docker.push \
		docker.tar docker.untar
