#!/bin/bash
# This tests all possible breakpoints, will take a while...
full_archive="foo.tar.test"
target/release/tarlive --input test_data/test.files > ${full_archive}
max_position=$(wc -c <"${full_archive}")
md5_expected=$(cat "${full_archive}" | md5)
for ((first=1; first<=max_position; first++)); do
    after=$((first + 1))
    md5sum=$( ( target/release/tarlive --input test_data/test.files | head -c $first ; target/release/tarlive --input test_data/test.files --offset $after ) | md5 )
    if [ "${md5_expected}" != "${md5sum}" ]
    then
        echo "${first}  ${md5_expected} ${md5sum}"
    fi
done