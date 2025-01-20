#!/bin/sh
#
# The script to run ansible to update current backend
# Requires toml cargo package

remote_user="${remote_user:-wawrzek_ivynet_dev}"
echo " Remote user is: ${remote_user}"

echo "Find the backend version in Cargo.toml"
version_backend=$(toml get ../../backend/Cargo.toml package.version |tr -d [\"])
version_scraper=$(toml get ../../scraper/Cargo.toml package.version |tr -d [\"])
echo " Backend Version is: ${version_backend}"
echo " Scraper Version is: ${version_scraper}"
echo "Activate Ansible (if necessary)"
[ -f "$HOME/bin/ansible/bin/activate" ] && source $HOME/bin/ansible/bin/activate

cat << EOF
Run the playbook


 ansible-playbook \\
  -i gcp.yml \\
  -u ${remote_user} \\
  -e "ivynet_backend_release=${version_backend}" \\
  -e "ivynet_scraper_release=${version_scraper}" \\
  --vault-password-file ~/.vault.txt \\
  backend.yml
EOF

ansible-playbook -i gcp.yml -u ${remote_user} -e "ivynet_backend_release=${version_backend} ivynet_scraper_release=${version_scraper}" --vault-password-file ~/.vault.txt backend.yml
