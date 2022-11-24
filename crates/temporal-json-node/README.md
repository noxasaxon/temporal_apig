# temporal-json-node
[![npm-ci](https://github.com/noxasaxon/temporal_apig/actions/workflows/npm-ci.yml/badge.svg)](https://github.com/noxasaxon/temporal_apig/actions/workflows/npm-ci.yml)

NodeJS bindings to the [Temporal API Gateway JSON Encoder](../temporal-json/README.md)


## Commands

### Build
```shell
cd crates/temporal-node-json
yarn install
yarn build
yarn test
```

### Publish
```shell
npm version patch
git tag -a <version> -m "<tag_message>"
git push --follow-tags
```
then commit and push