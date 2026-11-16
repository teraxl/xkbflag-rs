#!/bin/bash

suffix="$1"
basedir="$(dirname $(readlink -f $0))/.."
target="$basedir/target"
version="v0.1.0"
arch="$(uname -m)"
apps="xkbflag-rs"
assets="xkbflag-rs.desktop install.sh"
triple="$arch-unknown-linux-gnu"

case $arch in
    aarch64)
      build_arch=arm64
      ;;
    *)
      build_arch=$arch
      ;;
esac

get_alt_branch() {
    local branch=$(grep ALT_BRANCH_ID /etc/os-release | cut -d'=' -f2 | tr -d '"')
    local result
    case "$branch" in
        p10) result="alt1" ;;
        p11) result="alt2" ;;
        *) result="$branch" ;;
    esac
    echo "$result"
}

create_run() {
    echo "Packaging .run for $build_arch"

    name="xkbflag-rs-${version}${suffix}-linux-$build_arch"

    if [ ! -f "$target/$triple/lto/xkbflag-rs" ]; then
        return
    fi

    rm -rf "$target/$name"
    mkdir "$target/$name"

    for app in $apps; do
        if ! cp "$target/$triple/lto/$app" "$target/$name/"; then
            exit 1
        fi
    done

    for asset in $assets; do
        cp "$basedir/package/$asset" "$target/$name/"
    done

    cd "$target"
    tar c "$name" | xz -9 > "$name.tar.xz"

    makeself --quiet --tar-quietly --xz --needroot --sha256 "$name" "$name.run" "xkbflag-rs version $version" ./install.sh
}

create_rpm() {
    echo "Packaging .rpm for $build_arch"

    rpm_version="$(get_alt_branch)"
    name="xkbflag-rs-${version}-${rpm_version}-$build_arch"
    tmpdir="$(mktemp -d)"
    rpm="$tmpdir/rpm"

    export RPM_BUILDROOT="$rpm/root"

    mkdir -p "$rpm/BUILD"
    mkdir -p "$rpm/RPMS"
    mkdir -p "$rpm/SOURCES"
    mkdir -p "$rpm/SPECS"
    mkdir -p "$rpm/SRPMS"
    mkdir -p "$rpm/BUILDROOT"

    sed "s/{{version}}/$version/;s/{{arch}}/$arch/;s/{{release}}/$rpm_version/" \
        "$basedir/package/rpm/package.spec.in" > "$rpm/SPECS/package.spec"

    mkdir -p "$RPM_BUILDROOT/usr/bin"
    mkdir -p "$RPM_BUILDROOT/usr/share/applications"

    for app in $apps; do
      install -m 755 "$target/$triple/lto/$app" "$RPM_BUILDROOT/usr/bin/"
    done
    cp "$basedir/package/xkbflag-rs.desktop" "$RPM_BUILDROOT/usr/share/applications/"

    if ! rpmbuild --define "_topdir $rpm" \
             --define "_buildroot $rpm/BUILDROOT" \
             --buildroot "$rpm/BUILDROOT" \
             -bb "$rpm/SPECS/package.spec"; then
      exit 1
    fi

    cp "$rpm/RPMS/$arch"/*.rpm "$target/$name.rpm"

    rm -rf "$tmpdir"
}

create_deb() {
    echo "Packaging .deb for $build_arch"

    name="xkbflag-rs-${version}${suffix}-$build_arch"
    tmpdir="$(mktemp -d)"
    deb="$tmpdir/deb"

    mkdir -p "$deb/DEBIAN"
    mkdir -p "$deb/usr/bin"
    mkdir -p "$deb/usr/share/applications"

    for app in $apps; do
        install -m 755 "$target/$triple/lto/$app" "$deb/usr/bin/"
    done
    cp "$basedir/package/xkbflag-rs.desktop" "$deb/usr/share/applications/"

    sed "s/{{version}}/$version/;s/{{arch}}/amd64/" \
        "$basedir/package/debian/control.in" > "$deb/DEBIAN/control"

    cp "$basedir/package/debian/preinst" "$deb/DEBIAN/preinst"
    cp "$basedir/package/debian/postinst" "$deb/DEBIAN/postinst"
    cp "$basedir/package/debian/prerm" "$deb/DEBIAN/prerm"
    chmod 755 "$deb/DEBIAN/preinst" "$deb/DEBIAN/postinst" "$deb/DEBIAN/prerm"

    dpkg-deb --build --root-owner-group "$deb" "$target/$name.deb"

    rm -rf "$tmpdir"
}

create_rpm
#create_deb
