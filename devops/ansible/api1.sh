#!/bin/bash
#

while getopts b:s:u: flag
do
    case "${flag}" in
        b) BACKEND=${OPTARG};;
        s) SCANNER=${OPTARG};;
        u) USERNAME=${OPTARG};;
    esac
done

if [[ "${BACKEND}x" == "x" || "${SCANNER}x" == "x" ]]
then
	echo "One of the version not specified"
	exit 2
fi


USERNAME="${USERNAME:-wawrzek_ivynet_dev}"

ansible-playbook -i gcp.yml -u ${USERNAME}  --vault-password-file ~/.vault.txt -e "ivynet_scraper_release=${SCANNER}" -e "ivynet_backend_release=${BACKEND}" api1.yml
