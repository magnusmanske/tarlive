#!/bin/bash
# This tests all possible breakpoints, will take a while...
full_archive="foo.tar.test"
target/release/tarlive --input test_data/test.files > ${full_archive}

# Check full archive
mkdir tmp
cp "${full_archive}" tmp
cd tmp
tar -xf "${full_archive}"
checksum_unpacked=$(md5 test_data/* | grep -v files | sort | md5)
cd ..
rm -rf tmp
checksum_original=$(md5 test_data/* | grep -v files | sort | md5)
if [ "${checksum_unpacked}" != "${checksum_original}" ]
then
    echo "${full_archive} has faulty content"
    exit 0
fi
echo "${full_archive} is valid"

max_position=$(wc -c <"${full_archive}")
md5_expected=$(cat "${full_archive}" | md5)
echo "STARTING"
# 512513
# 811520
# 636581??
for ((first=0; first<=max_position; first++)); do
    after=$((first + 1))
    md5sum=$( ( target/release/tarlive --input test_data/test.files | head -c $first ; target/release/tarlive --input test_data/test.files --offset $after ) | md5 )
    if [ "${md5_expected}" != "${md5sum}" ]
    then
        echo "${first}  ${md5_expected} ${md5sum}"
        exit 0
    fi
done