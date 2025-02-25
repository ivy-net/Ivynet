#!/bin/bash
#
# The script to run ansible to update current api
# Requires toml cargo package


services="api ingress scraper"
declare version

pre_checks() {
  echo "Check if remote user is defined"
  remote_user="${remote_user:-wawrzek_ivynet_dev}"
  echo " Remote user is: ${remote_user}"

  echo "Activate Ansible (if necessary)"
  [ -f "$HOME/bin/ansible/bin/activate" ] && (source $HOME/bin/ansible/bin/activate; echo " Activated") || echo " Not needed"
}


find_version() {
  echo "Find the ${1} version in Cargo.toml"
  version=$(toml get ../../${1}/Cargo.toml package.version |tr -d [\"])
  echo " ${1} Version is: ${version}"
}


one_service() {
  find_version "${1}"
  version_service=${version}
  cat << EOF
Run the playbook
 ansible-playbook \\
   -i gcp.yml \\
   -u ${remote_user} \\
   -e "ivynet_${1}_release=${version_service}" \\
   --vault-password-file ~/.vault.txt \\
   ${1}.yml
EOF
ansible-playbook -i gcp.yml -u ${remote_user} -e "ivynet_${1}_release=${version_service}" --vault-password-file ~/.vault.txt ${1}.yml
}

all_services() {

  find_version "api"
  version_api=${version}
  find_version "ingress"
  version_ingress=${version}
  find_version "scraper"
  version_scraper=${version}
  cat << EOF
Run the playbook

 ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  -e "ivynet_api_release=${version_api}" \\
  -e "ivynet_ingress_release=${version_ingress}" \\
  -e "ivynet_scraper_release=${version_scraper}" \\
  --vault-password-file ~/.vault.txt \\
  api.yml
EOF

  ansible-playbook \
    -i gcp.yml \
    -u ${remote_user} \
    -e "ivynet_api_release=${version_api}" \
    -e "ivynet_ingress_release=${version_ingress}" \
    -e "ivynet_scraper_release=${version_scraper}" \
    --vault-password-file ~/.vault.txt \
    api.yml
}

# MAIN PART
if [[ $# -eq 0 ]]
then
  pre_checks
  all_services
elif [[ $# -gt 1 ]]
then
  echo "Only one parametrs allowed"
  exit 2
else
  if [[ ${services} =~ ${1} ]]
  then
    pre_checks
    one_service ${1}
  else
    echo "Wrong service called. Has to be one of"
    echo ${services}
    echo ""
    echo "But it was ${1}"
  fi
fi
