Name:           reflector
Version:        0.1.0
Release:        1%{?dist}
Summary:        PacketParamedic Reflector - Self-hosted Network Test Endpoint
License:        BlueOak-1.0.0
URL:            https://github.com/packetparamedic/packetparamedic

Source0:        reflector
Source1:        reflector.service
Source2:        reflector.toml.example

BuildArch:      x86_64

Requires:       iperf3

%description
PacketParamedic Reflector is a single-binary, cryptographically-identified
endpoint that exposes a zero-trust control plane and a tightly-scoped data
plane (throughput + latency reflector). It is designed to be safe to run on
the public Internet without becoming an open relay.

%install
# Binary
install -D -m 0755 %{SOURCE0} %{buildroot}%{_prefix}/local/bin/reflector

# systemd unit
install -D -m 0644 %{SOURCE1} %{buildroot}%{_unitdir}/reflector.service

# Configuration
install -D -m 0644 %{SOURCE2} %{buildroot}%{_sysconfdir}/reflector/reflector.toml

# State directory
install -d -m 0700 %{buildroot}%{_sharedstatedir}/reflector

%post
%systemd_post reflector.service
# Enable and start on fresh install
if [ $1 -eq 1 ]; then
    systemctl daemon-reload
    systemctl enable reflector.service
    systemctl start reflector.service
fi

%preun
%systemd_preun reflector.service
if [ $1 -eq 0 ]; then
    systemctl stop reflector.service
    systemctl disable reflector.service
fi

%postun
%systemd_postun_with_restart reflector.service

%files
%{_prefix}/local/bin/reflector
%{_unitdir}/reflector.service
%config(noreplace) %{_sysconfdir}/reflector/reflector.toml
%dir %attr(0700, root, root) %{_sharedstatedir}/reflector

%changelog
* Wed Feb 12 2026 PacketParamedic Team <dev@packetparamedic.io> - 0.1.0-1
- Initial RPM release
- mTLS control plane on port 4000
- Ed25519 cryptographic identity
- iperf3 throughput engine
- UDP echo latency reflector
- Hardened systemd unit with DynamicUser
