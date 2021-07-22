#!/bin/bash

cpudirs=$(find "/sys/devices/system/cpu/" -maxdepth 1 -name cpu[0-9]*)

list="level type size coherency_line_size number_of_sets"
dict_string="'{"

for dir in $cpudirs; do
    index_dirs=$(find "$dir/cache/" -maxdepth 1 -name index[0-9]*)
    for index in $index_dirs; do
        for file in $list; do
            val=$(cat $index/$file)
            dict_string="$dict_string\"$index/$file\": \"$val\", "
            done
        done
done

final_dict=${dict_string::-2}
echo "$final_dict}'"