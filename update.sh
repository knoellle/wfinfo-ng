#!/usr/bin/env sh
curl https://api.warframestat.us/wfinfo/prices/ | jq . > prices.json
curl https://api.warframestat.us/wfinfo/filtered_items/ | jq . > filtered_items.json
