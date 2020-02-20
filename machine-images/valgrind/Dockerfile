FROM ubuntu:18.04

RUN apt-get -y update

# create volume
RUN mkdir -p /root/build
VOLUME /root/build

# Fix non interactive bug in Tzdata https://bugs.launchpad.net/ubuntu/+source/tzdata/+bug/1554806
RUN ln -fs /usr/share/zoneinfo/Europe/Copenhagen /etc/localtime
RUN apt -y install build-essential valgrind git-all curl libssl-dev pkg-config
RUN apt -y clean
# Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH "$PATH:/root/.cargo/bin"


# Now use docker build -t rust-valgrind . && docker run -v `pwd`:/repo -it rust-valgrind
