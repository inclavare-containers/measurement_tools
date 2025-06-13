%define alinux_release 1
%global config_dir /etc/runtime_measurer

Name:           runtime_measurer
Version:        0.1.0
Release:        %{alinux_release}%{?dist}
Summary:        Runtime measurement tool for confidential computing environments
Group:          Applications/System
BuildArch:      x86_64

License:        Apache-2.0
URL:            https://github.com/inclavare-containers/measurement_tools
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  protobuf-compiler
BuildRequires:  protobuf-devel

Requires:       attestation-agent

%global debug_package %{nil}
%global _build_id_links none

%global __requires_exclude_from ^%{_bindir}/.*$

%description
Runtime Measurer is a flexible runtime measurement tool for confidential 
computing environments that measures various system resources and communicates 
with Attestation Agents via ttrpc protocol. It supports file measurements, 
process measurements, and container image measurements.

%prep
%autosetup -n runtime_measurer-%{version}

%build
export CARGO_HOME=%{_builddir}/.cargo
export RUSTFLAGS="-C opt-level=3 -C target-cpu=native"

cargo build --release --locked

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_sysconfdir}/runtime_measurer
mkdir -p %{buildroot}%{_unitdir}

install -m 755 target/release/runtime_measurer %{buildroot}%{_bindir}/runtime_measurer
install -m 644 config.example.toml %{buildroot}%{_sysconfdir}/runtime_measurer/config.toml
install -m 644 runtime_measurer.service %{buildroot}%{_unitdir}/runtime_measurer.service

%files
%doc README.md
%{_bindir}/runtime_measurer
%config(noreplace) %{_sysconfdir}/runtime_measurer/config.toml
%{_unitdir}/runtime_measurer.service

%post
%systemd_post runtime_measurer.service

%preun
%systemd_preun runtime_measurer.service

%postun
%systemd_postun_with_restart runtime_measurer.service

%changelog
* Fri May 30 2025 Weidong Sun <sunweidong@linux.alibaba.com> - 0.1.0-1
- Initial package release
- Runtime measurement tool for confidential computing
- Support for file measurements
- Integration with attestation-agent via ttrpc protocol 