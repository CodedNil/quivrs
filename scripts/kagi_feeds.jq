def filters:
  $config[0];

def is_not_in($blacklist):
  . as $value
  | ($blacklist | index($value) | not);

def contains_no_blacklisted_substrings($blacklist):
  . as $value
  | ($blacklist | any(. as $blocked | ($value | ascii_downcase) | contains($blocked | ascii_downcase)) | not);

def host:
  capture("^[a-zA-Z][a-zA-Z0-9+.-]*://(?<host>[^/:?#]+)")?.host
  | ascii_downcase;

def host_has_no_blacklisted_suffixes($blacklist):
  . as $url
  | ($url | host) as $host
  | ($host == null or ($blacklist | any(. as $blocked | $host | endswith($blocked | ascii_downcase)) | not));

def is_in($allowlist):
  . as $value
  | ($allowlist | index($value));

del(.[]?.display_names)
| with_entries(
    select(
      (.key | is_not_in(filters.feed_name_blacklist))
      and (.key | contains_no_blacklisted_substrings(filters.feed_name_contains_blacklist))
    )
  )
| with_entries(
    select(.value.category_type | is_not_in(filters.category_type_blacklist))
  )
| with_entries(
    select(.value.source_language | is_in(filters.source_language_allowlist))
  )
| walk(
    if type == "object" and (.feeds? | type == "array") then
      .feeds |= map(
        select(
          (. | contains_no_blacklisted_substrings(filters.feed_item_contains_blacklist))
          and (. | host_has_no_blacklisted_suffixes(filters.feed_item_host_suffix_blacklist))
        )
      )
    else
      .
    end
  )
