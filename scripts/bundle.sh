#! /usr/bin/env bash
#
# This script will bundle suricata-update for you.
#
# To use, run from the top Suricata source directory:
#
#    ./scripts/bundle.sh

what="$1"

while IFS= read -r requirement; do
    set -- $requirement

    # If a requirement was specified on the command line, skip all other
    # requirements.
    if [ "${what}" != "" ]; then
        if [ "${what}" != "$1" ]; then
            continue
        fi
    fi
    case "$1" in
        suricata-update)
            repo=${SU_REPO:-$2}
            branch=${SU_BRANCH:-$3}
            echo "===> Bundling ${repo} -b ${branch}"
            rm -rf suricata-update.tmp
            git clone "${repo}" -b "${branch}" suricata-update.tmp
            cp -a suricata-update.tmp/* suricata-update/
            rm -rf suricata-update.tmp
            ;;
        \#*)
            # Ignore comment.
            ;;
        "")
            # Ignore blank line.
            ;;
        *)
            echo "error: unknown requirement: $1"
            ;;
    esac
done < ./requirements.txt
