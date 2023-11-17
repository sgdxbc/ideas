#!/bin/bash -ex
date=$(TZ=Asia/Singapore date -R)
cat << END > content/default/$(date +%s).md
---
date: ${date}
---
END