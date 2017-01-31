test $TRAVIS_BRANCH = "master" && \
cargo doc --package allocators --no-deps && \
cargo doc --package chunked --no-deps && \
cargo doc --package compact --no-deps && \
cargo doc --package compact_macros --no-deps && \
cargo doc --package kay --no-deps && \
cargo doc --package kay_macros --no-deps && \
echo '<meta http-equiv=refresh content=0;url=kay/index.html>' > target/doc/index.html && \
sudo pip install ghp-import && \
ghp-import -n target/doc && \
git push -qf https://${GH_TOKEN}@github.com/${TRAVIS_REPO_SLUG}.git gh-pages