#!/bin/bash
#

source .env

while getopts e:o:p: flag
do
    case "${flag}" in
        e) EMAIL=${OPTARG};;
        o) ORG=${OPTARG};;
        p) PASSWORD=${OPTARG};;
    esac
done

if [[ "${EMAIL}x" == "x" || "${ORG}x" == "x" ]]
then
	echo "Email or Org not specified"
	exit 2
fi

ivynet-backend --add-organization ${EMAIL}:${PASSWORD}/${ORG} --db-uri postgresql://ivy:${PGPASSWORD}@${HOST}:5432/ivynet
