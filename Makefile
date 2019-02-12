#Magic. Reads env variables from .env file
#See: https://unix.stackexchange.com/questions/235223/makefile-include-env-file
include .env
export $(shell sed 's/=.*//' .env)

run:
	RUST_BACKTRACE=1 DB_CONN_STRING="postgres://postgres:7713659@127.0.0.1:5432/quizzicaldb" cargo run

docker-build:
	docker build -t $(IMAGE_NAME):$(TAG) .

docker-publish:
	docker push $(IMAGE_NAME):$(TAG)

docker-start-dev:
	docker-compose -f docker-compose.development.yml up -d

docker-stop-dev:
	docker-compose -f docker-compose.development.yml stop
	docker-compose -f docker-compose.development.yml rm

docker-start-prod:
	docker-compose -f docker-compose.production.yml up -d

docker-stop-prod:
	docker-compose -f docker-compose.production.yml stop
	docker-compose -f docker-compose.production.yml rm