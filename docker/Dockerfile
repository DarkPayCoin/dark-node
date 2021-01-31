FROM phusion/baseimage:0.11 as builder
LABEL maintainer "admin@darkdot.network"

WORKDIR /darkdot

COPY . /darkdot

RUN apt-get update && \
	apt-get upgrade -y && \
	apt-get install -y cmake pkg-config libssl-dev git clang
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
        export PATH=$PATH:$HOME/.cargo/bin && \
        scripts/init.sh && \
        cargo build --release

FROM phusion/baseimage:0.11
LABEL maintainer "admin@darkdot.network"

COPY --from=builder /darkdot/target/release/dark-node /usr/local/bin

RUN mv /usr/share/ca* /tmp && \
	rm -rf /usr/share/*  && \
	mv /tmp/ca-certificates /usr/share/ && \
	rm -rf /usr/lib/python* && \
	useradd -m -u 1000 -U -s /bin/sh -d /darkdot darkdot && \
	mkdir -p /darkdot/.local/share/dark-node && \
	chown -R darkdot:darkdot /darkdot/.local && \
	ln -s /darkdot/.local/share/dark-node /data && \
	rm -rf /usr/bin /usr/sbin

USER darkdot
EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/dark-node"]
