#!/bin/bash

VERSION=$1
gcloud storage cp "gs://ivynet-share-test/ivynet-${VERSION}*" gs://ivynet-share/
