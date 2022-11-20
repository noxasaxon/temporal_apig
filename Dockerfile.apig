FROM rust:1.65.0-buster as base

# install protoc for the temporal rust sdk's protobuffers
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive \
    apt-get install --assume-yes \
    protobuf-compiler


FROM base as app_builder

WORKDIR /app

ARG CARGO_HOME='/app/cargo'
ARG CARGO_TARGET_DIR='/app/target'
ARG CARGO_INCREMENTAL=0

COPY . .

RUN USER=root cargo build --locked --bin apig_server --release


# From this line onwards, we're in a new image, which will be the image used in production
FROM debian:buster-slim

# Create a group and user
RUN addgroup appgroup && adduser appuser && adduser appuser appgroup

# Copy over the build artifact from the previous step
WORKDIR /home/appuser
COPY --from=app_builder ./app/target/release/apig_server .
RUN chown -R appuser: ./apig_server

# Tell docker that all future commands should run as appuser
USER appuser

# Set the locale
ENV LANG C.UTF-8
ENV LC_ALL C.UTF-8

# Run the app
ENTRYPOINT ["./apig_server"]
