INSTALL_LOC="/usr/local/bin"

echo "You are installing the calyx-lsp program"
echo "========================================"

echo "0. Checks:"
grep -q 'name = "calyx"' ../Cargo.toml 1>/dev/null 2>/dev/null
if [ $? -ne 0 ] ; then
    echo "   => Please run this install script from the calyx repository ❌"
    exit 1
else
    echo "   => You are installing this from the calyx repository ✅"
fi

if which calyx-lsp >/dev/null; then
    echo "   => calyx-lsp is already installed under $(which calyx-lsp) ❌"
    exit 1
else
    echo "   => This is a fresh install ✅"
fi

echo
echo "1. Configure install location:"
echo "   => Default: under '$INSTALL_LOC'"
printf "   => Are you ok with this? (y/n) "
read answer
if [ "$answer" != "y" ]; then
    printf "   => Please enter the new install location: "
    read INSTALL_LOC
fi
echo
echo "2. Confirm install location:"
printf "   => Confirm that you want to install under '$INSTALL_LOC' (y/n) "
read answer
if [ "$answer" != "y" ]; then
    echo
    echo "Exiting..."
    exit 0
fi
echo
cargo build --manifest-path Cargo.toml || exit 1
OLD_PWD=`pwd`
cd ..
REPO=`pwd`
cd $INSTALL_LOC
sudo ln -s "$REPO/target/debug/calyx-lsp" calyx-lsp \
    && echo "Installation was successful!" \
    || sudo rm -f calyx-lsp
cd $OLD_PWD
