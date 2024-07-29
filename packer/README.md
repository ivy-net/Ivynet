# General information

## Setting up VM

The VM can be created with a gcloud command.
See example below.
The VM type (n2-standard-2) in the example is cheap, but slow when building rust projects.

* Prepare a few variables. First VM name,
```
VMNAME=ivy-c
```
* next version of cloudstation (I plan to find a way to call 'latest')
```
VERSION=4
```
* and finally the tag (or tags) to set firewall
```
TAG=holesky-eigenda
```
* build machine
```
gcloud compute instances create --image ivynet-cloudstation-$VERSION  --zone "us-central1-a" --boot-disk-size 40GB --machine-type n2-standard-2  $VMNAME
```
* add the tag to open the firewall
```
gcloud compute instances add-tags $VMNAME --tags $TAG
```

### SSH
THE GCP CLI can be also use to login to the VM (because we set OS Login: (https://cloud.google.com/compute/docs/oslogin))
```
gcloud compute ssh --zone "us-central1-a"  --project "ivynet-tests" $VMNAME
```
As well as coping files. E.g.:
```
gcloud compute scp  --zone "us-central1-a"  --project "ivynet-tests" metadata.json  $VMNAME:
```
The VM should have a network tag _holeksy-eigenda_ to enable firewall setup.
I don't know how to do this from commandline, yet.


## On the machine
* After login change the ownership of the '/opt/eigen' directory, if you plan to build any programs.
```
sudo chown -R $USER /opt/eigen

```
* The `/opt/eigen/bin` directory is added to the $PATH variable, so it's worth to drop the `ivynet-cli` there.

_There are more info in the MOTD section._
_This section is also printed after login (as MOTD)._

# Packer build notes

* Download plugin for GCP VM
```
packer init cloudstation.pkr.hcl
```
* You need a private key for packer to clone ivynet repo.
* To build pass the version number e.g.
```
packer build -var 'version=2' cloudstation.pkr.hcl
```
## Ansible

### Molecule
* In test pass '--tags github' to avoid downloading g1, g2 files.
* For molecule it can be:
```
molecule converge -- --tags github
```

## MOTD
Also in the motd file.
===============================================================================
This is a cloudstation VM dedicated to work with IvyNet Client.

It based on Ubuntu and includes docker, rust and cargo packages.

The code of:
* eigenda,
* eigenda-operator-setup,
* ivynet
are cloned to the /opt/eigen folder.

Additionally, the
* eigen-cli
is downloaded to the /opt/eigen/bin folder.

Finally the
* g1,
* g2
files are in the resources subfolder of the eigenda-operator-setup repository.
===============================================================================
