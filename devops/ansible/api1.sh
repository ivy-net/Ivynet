#!/bin/bash
#

while getopts b:i:s:u: flag
do
    case "${flag}" in
        b) BACKEND=${OPTARG};;
        i) INGRESS=${OPTARG};;
        s) SCANNER=${OPTARG};;
        u) USERNAME=${OPTARG};;
    esac
done

if [[ "${BACKEND}x" == "x" || "${SCANNER}x" == "x" || "${INGRESS}x" == "x" ]]
then
	echo "One of the version not specified"
	exit 2
fi


USERNAME="${USERNAME:-wawrzek_ivynet_dev}"

sed -i.bak "s/gcp_area_backend:&gcp_env_gha/api1/" api.yml

ansible-playbook -i gcp.yml \
  -u ${USERNAME} \
  --vault-password-file ~/.vault.txt \
  -e "ivynet_api_release=${BACKEND}" \
  -e "ivynet_ingress_release=${INGRESS}" \
  -e "ivynet_scanner_release=${SCANNER}" \
  api.yml

mv api.yml.bak api.yml
