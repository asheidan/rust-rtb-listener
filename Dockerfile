FROM scratch

COPY target/x86_64-unknown-linux-musl/release/rtb-listener .

ENTRYPOINT ["./rtb-listener"]
