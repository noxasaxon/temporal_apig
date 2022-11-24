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
git tag -a <version> -m "<version>"
git push --follow-tags
```

**Notes**
- Publish will only occur if the commit message is just the version number ex.  1.0.0
- Changes to some files like `*.md` won't trigger the CI pipeline.