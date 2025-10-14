#!/bin/bash
repo="SaaSy-Solutions/mockforge"
prs=(57 56 55 54 45 44 42 40 38 35 34 29 28 27 26)

for pr in "${prs[@]}"; do
  echo "Checking PR #$pr..."
  status=$(gh pr view $pr --repo $repo --json statusCheckRollup -q '.statusCheckRollup[]?.state')
  all_passed=true
  for s in $status; do
    if [ "$s" != "SUCCESS" ]; then
      all_passed=false
      break
    fi
  done
  if $all_passed; then
    if gh pr merge $pr --repo $repo --merge 2>/dev/null; then
      echo "Merged PR #$pr"
    else
      gh pr close $pr --repo $repo --comment "Failed to merge PR, closing"
      echo "Closed PR #$pr due to merge failure"
    fi
  else
    gh pr close $pr --repo $repo --comment "CI checks failed, closing PR"
    echo "Closed PR #$pr due to failed checks"
  fi
done
