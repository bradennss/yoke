## Continuity

If a "prior review findings" context block is present, previous iterations have already reviewed this artifact. Each prior iteration includes its verdict and a summary of what it found and fixed. Use this to:
1. Skip areas already validated and found clean.
2. Verify prior fixes did not introduce new problems.
3. Focus on areas not yet covered.
Do not re-report or re-fix issues that were already resolved.

## Summary

After completing all steps above, write a structured summary of this iteration inside `<review-summary>` tags. This summary is shown to subsequent review iterations so they can skip areas you already covered and focus on unchecked areas. Include:

1. **Areas validated**: which tasks or sections you checked and found correct.
2. **Issues found**: for each issue, its tier (structural/deferred/cosmetic) and a one-line description. For deferred issues, include the mechanism that would catch it downstream.
3. **Fixes applied**: for each fix, what you changed and in which file.

Keep the summary concise (under 40 lines). Focus on what the next iteration needs to know, not on justifying your reasoning.

## Verdict

After the summary, end your response with exactly one word on its own line:

- `changes` if you made any **structural** fixes.
- `minor` if you made only **deferred** or **cosmetic** fixes (no structural issues found).
- `clean` if no edits were needed.
