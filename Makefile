###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)




######################
# Project parameters #
######################

NAME := baza
OWNER := $(or $(GITHUB_REPOSITORY_OWNER),instrumentisto)
REGISTRIES := $(strip $(subst $(comma), ,\
	$(shell grep -m1 'registry: \["' .github/workflows/ci.yml \
	        | cut -d':' -f2 | tr -d '"][')))
VERSION ?= $(strip $(shell grep -m1 'version = "' Cargo.toml | cut -d'"' -f2))

RUST_VER := $(strip $(shell grep -m1 'RUST_VER: ' .github/workflows/ci.yml \
                            | cut -d':' -f2 | tr -d '"'))




###########
# Aliases #
###########

all: fmt lint test.unit


docs: cargo.doc


fmt: cargo.fmt


image: docker.image


lint: cargo.lint


test: test.e2e test.unit




####################
# Running commands #
####################

# Stop running project in local environment.
#
# Usage:
#	make down [dockerized=(no|yes)]

down:
ifeq ($(dockerized),yes)
	-docker stop $(NAME)
else
	$(eval pid := $(shell ps ax | grep -v grep | grep 'target/' | grep '/baza'))
	$(if $(call eq,$(pid),),,kill $(strip $(word 1,$(pid))))
endif


# Run project in local environment.
#
# Usage:
#	make up [background=(no|yes)]
#	        [debug=(yes|no)]
#	        [( [dockerized=no]
#	         | dockerized=yes [tag=(dev|<docker-tag>)]
#	           [( [rebuild=no] | rebuild=yes [no-cache=(no|yes)] )] )]

up: down
ifeq ($(dockerized),yes)
ifeq ($(rebuild),yes)
	@make docker.image tag=$(tag) debug=$(debug) no-cache=$(no-cache)
endif
	@mkdir -p .cache/baza/
	docker run --rm $(if $(call eq,$(background),yes),-d,-it) --name $(NAME) \
	           -u $(shell id -u) \
	           -p 9294:9294 \
	           -v "$(PWD)/.cache/baza/":/.cache/baza/:z \
		$(OWNER)/$(NAME):$(or $(tag),dev) -r .cache/baza
else
	cargo run $(if $(call eq,$(debug),no),--release,) -- -r .cache/baza \
		$(if $(call eq,$(background),yes),&,)
endif




##################
# Cargo commands #
##################

cargo-crate = $(if $(call eq,$(crate),),--workspace,-p $(crate))


# Generate crates documentation from Rust sources.
#
# Usage:
#	make cargo.doc [crate=<crate-name>] [private=(yes|no)]
#	               [open=(yes|no)] [clean=(no|yes)]

cargo.doc:
ifeq ($(clean),yes)
	@rm -rf target/doc/
endif
	cargo doc $(cargo-crate) --all-features \
		$(if $(call eq,$(private),no),,--document-private-items) \
		$(if $(call eq,$(open),no),,--open)


# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Lint Rust sources with Clippy.
#
# Usage:
#	make cargo.lint [crate=<crate-name>]

cargo.lint:
	cargo clippy $(cargo-crate) --all-features -- -D warnings




####################
# Testing commands #
####################

# Run project E2E tests.
#
# Usage:
#	make test.e2e [only=<regex>]
#	              [( [start-app=no]
#	               | start-app=yes [debug=(yes|no)]
#	                 [( [dockerized=no]
#	                    | dockerized=yes [tag=(dev|<docker-tag>)]
#	                      [( [rebuild=no] |
#	                         rebuild=yes [no-cache=(no|yes)] )] )] )]

test.e2e:
ifeq ($(start-app),yes)
	@make up background=yes debug=$(debug) \
	         dockerized=$(dockerized) tag=$(tag) \
	         rebuild=$(rebuild) no-cache=$(no-cache)
	sleep 5
endif
	cargo test -p baza-e2e --test e2e -- -vv \
		$(if $(call eq,$(only),),,--name '$(only)')
ifeq ($(start-app),yes)
	@make down dockerized=$(dockerized)
endif


# Run project unit tests.
#
# Usage:
#	make test.unit [crate=<crate-name>]

test.unit:
	cargo test \
		$(if $(call eq,$(crate),),--workspace --exclude baza-e2e,-p $(crate)) \
		--all-features




###################
# Docker commands #
###################

docker-registries = $(strip $(if $(call eq,$(registries),),\
                            $(REGISTRIES),$(subst $(comma), ,$(registries))))
docker-tags = $(strip $(if $(call eq,$(tags),),\
                      $(VERSION),$(subst $(comma), ,$(tags))))


# Build project Docker image.
#
# Usage:
#	make docker.image [tag=(dev|<docker-tag>)] [no-cache=(no|yes)]
#	                  [debug=(yes|no)]

github_url := $(strip $(or $(GITHUB_SERVER_URL),https://github.com))
github_repo := $(strip $(or $(GITHUB_REPOSITORY),$(OWNER)/$(NAME)))

docker.image:
	docker build --network=host --force-rm \
		$(if $(call eq,$(no-cache),yes),--no-cache --pull,) \
		--build-arg rust_ver=$(RUST_VER) \
		--build-arg rustc_mode=$(if $(call eq,$(debug),no),release,debug) \
		--build-arg rustc_opts=$(if $(call eq,$(debug),no),--release,) \
		--label org.opencontainers.image.source=$(github_url)/$(github_repo) \
		--label org.opencontainers.image.revision=$(strip \
			$(shell git show --pretty=format:%H --no-patch)) \
		--label org.opencontainers.image.version=$(strip $(VERSION)) \
		-t $(OWNER)/$(NAME):$(or $(tag),dev) ./
# TODO: Enable after first release.
#		--label org.opencontainers.image.version=$(strip \
#			$(shell git describe --tags --dirty))


# Manually push project Docker images to container registries.
#
# Usage:
#	make docker.push [tags=($(VERSION)|<docker-tag-1>[,<docker-tag-2>...])]
#	                 [registries=($(REGISTRIES)|<prefix-1>[,<prefix-2>...])]

docker.push:
	$(foreach tag,$(subst $(comma), ,$(docker-tags)),\
		$(foreach registry,$(subst $(comma), ,$(docker-registries)),\
			$(call docker.push.do,$(registry),$(tag))))
define docker.push.do
	$(eval repo := $(strip $(1)))
	$(eval tag := $(strip $(2)))
	docker push $(repo)/$(OWNER)/$(NAME):$(tag)
endef


# Tag project Docker image with the given tags.
#
# Usage:
#	make docker.tags [of=($(VERSION)|<docker-tag>)]
#	                 [tags=($(VERSION)|<docker-tag-1>[,<docker-tag-2>...])]
#	                 [registries=($(REGISTRIES)|<prefix-1>[,<prefix-2>...])]

docker.tags:
	$(foreach tag,$(subst $(comma), ,$(docker-tags)),\
		$(foreach registry,$(subst $(comma), ,$(docker-registries)),\
			$(call docker.tags.do,$(or $(of),$(VERSION)),$(registry),$(tag))))
define docker.tags.do
	$(eval from := $(strip $(1)))
	$(eval repo := $(strip $(2)))
	$(eval to := $(strip $(3)))
	docker tag $(OWNER)/$(NAME):$(from) $(repo)/$(OWNER)/$(NAME):$(to)
endef


# Save project Docker images to a tarball file.
#
# Usage:
#	make docker.tar [to-file=(.cache/docker/image.tar|<file-path>)]
#	                [tags=($(VERSION)|<docker-tag-1>[,<docker-tag-2>...])]

docker-tar-file = $(or $(to-file),.cache/docker/image.tar)

docker.tar:
	@mkdir -p $(dir $(docker-tar-file))
	docker save -o $(docker-tar-file) \
		$(foreach tag,$(subst $(comma), ,$(or $(tags),$(VERSION))),\
			$(OWNER)/$(NAME):$(tag))


# Load project Docker images from a tarball file.
#
# Usage:
#	make docker.untar [from-file=(.cache/docker/image.tar|<file-path>)]

docker.untar:
	docker load -i $(or $(from-file),.cache/docker/image.tar)




##################
# .PHONY section #
##################

.PHONY: all docs down fmt image lint test up \
        cargo.doc cargo.fmt cargo.lint \
        docker.image docker.tags docker.push docker.tar docker.untar \
        test.e2e test.unit
