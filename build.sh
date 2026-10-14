APP="Yomi"
WIN="x86_64-pc-windows-gnu"

# Create output directories
mkdir -p "./output/release/arch"
mkdir -p "./output/release/windows"
mkdir -p "./output/zip/"


# Default
cargo build --release
cp "target/release/${APP}" "./output/release/arch/${APP}"

cargo build --release --target "$WIN"
cp "target/${WIN}/release/${APP}.exe" "./output/release/windows/${APP}.exe"


# With Remote
cargo build --release --features "remote"
cp "target/release/${APP}" "./output/release/arch/${APP}WithRemote"

cargo build --release --target "$WIN" --features "remote"
cp "target/${WIN}/release/${APP}.exe" "./output/release/windows/${APP}WithRemote.exe"