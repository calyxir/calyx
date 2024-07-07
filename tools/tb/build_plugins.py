import os
import subprocess

for dir_name in os.listdir("plugins"):
    dir_path = os.path.join("plugins", dir_name)
    if os.path.isdir(dir_path):
        subprocess.run(
            [
                "cargo",
                "build",
                "--manifest-path",
                os.path.join(dir_path, "Cargo.toml"),
                "--release",
                "--target-dir",
                os.path.join(dir_path, "target"),
            ],
            check=True,
        )
        release_dir = os.path.join(dir_path, "target", "release")
        for file_name in os.listdir(release_dir):
            file_path = os.path.join(release_dir, file_name)
            if os.path.isfile(file_path) and (
                file_name.endswith(".dylib") or file_name.endswith(".so")
            ):
                dest_path = os.path.join("plugins", file_name)
                if os.path.isfile(dest_path):
                    os.remove(dest_path)
                os.rename(file_path, dest_path)
