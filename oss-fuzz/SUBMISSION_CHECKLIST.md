# OSS-Fuzz Submission Checklist

Use this checklist to ensure a smooth submission to OSS-Fuzz.

## Pre-Submission

- [ ] Project is open source with compatible license
- [ ] Project has significant user base or security requirements
- [ ] Maintainer contact information is up to date
- [ ] Project is actively maintained
- [ ] All fuzz targets are working locally

## Configuration Files

- [ ] `project.yaml` exists and is properly configured
  - [ ] `homepage` is correct
  - [ ] `primary_contact` email is valid and monitored
  - [ ] `main_repo` URL is correct
  - [ ] Sanitizers are appropriate for the project
  - [ ] `file_github_issue: true` is set (if using GitHub)

- [ ] `Dockerfile` is present and tested
  - [ ] Uses correct base image
  - [ ] Installs all required dependencies
  - [ ] Clones repository successfully
  - [ ] Copies build script

- [ ] `build.sh` is present and executable
  - [ ] Builds all fuzz targets successfully
  - [ ] Copies binaries to `$OUT` directory
  - [ ] Creates seed corpus archives (if available)
  - [ ] Copies dictionaries (if available)

## Fuzz Targets

- [ ] All fuzz targets compile successfully
- [ ] Fuzz targets are in correct locations
- [ ] Each target has a meaningful dictionary (optional but recommended)
- [ ] Seed corpus exists for each target (optional but recommended)
- [ ] Fuzz targets run without immediate crashes

## Local Testing

- [ ] Clone OSS-Fuzz repository:
  ```bash
  git clone https://github.com/google/oss-fuzz.git
  cd oss-fuzz
  ```

- [ ] Create project directory:
  ```bash
  mkdir projects/mockforge
  cp /path/to/mockforge/oss-fuzz/* projects/mockforge/
  ```

- [ ] Build Docker image:
  ```bash
  python infra/helper.py build_image mockforge
  ```
  **Expected**: Image builds without errors

- [ ] Build fuzz targets:
  ```bash
  python infra/helper.py build_fuzzers mockforge
  ```
  **Expected**: All targets build successfully

- [ ] Check build:
  ```bash
  python infra/helper.py check_build mockforge
  ```
  **Expected**: All checks pass

- [ ] Run each fuzz target:
  ```bash
  python infra/helper.py run_fuzzer mockforge fuzz_json_validator -- -max_total_time=60
  python infra/helper.py run_fuzzer mockforge fuzz_openapi_parser -- -max_total_time=60
  python infra/helper.py run_fuzzer mockforge fuzz_template_engine -- -max_total_time=60
  ```
  **Expected**: Targets run and make progress (executions/sec > 0)

- [ ] Test with different sanitizers:
  ```bash
  python infra/helper.py build_fuzzers --sanitizer address mockforge
  python infra/helper.py build_fuzzers --sanitizer memory mockforge
  python infra/helper.py build_fuzzers --sanitizer undefined mockforge
  ```
  **Expected**: Builds succeed with all sanitizers

- [ ] Generate coverage report:
  ```bash
  python infra/helper.py coverage mockforge
  ```
  **Expected**: Coverage report generated successfully

## Submission

- [ ] Fork OSS-Fuzz repository
- [ ] Create feature branch:
  ```bash
  git checkout -b add-mockforge
  ```

- [ ] Add project files:
  ```bash
  git add projects/mockforge
  ```

- [ ] Commit changes:
  ```bash
  git commit -m "Add MockForge project"
  ```

- [ ] Push to fork:
  ```bash
  git push origin add-mockforge
  ```

- [ ] Create pull request to google/oss-fuzz
  - [ ] Title: "Add MockForge project"
  - [ ] Description includes project information
  - [ ] Link to project repository
  - [ ] Mention maintainer availability

## Post-Submission

- [ ] Monitor pull request for feedback
- [ ] Address reviewer comments promptly
- [ ] Update integration files if requested
- [ ] Verify CI checks pass

## After Acceptance

- [ ] Set up email notifications
- [ ] Add OSS-Fuzz badge to README
- [ ] Monitor bug dashboard regularly
- [ ] Review and triage reported issues
- [ ] Keep integration files updated

## Email Configuration

- [ ] Primary contact email is monitored regularly
- [ ] Backup contact emails added (if available)
- [ ] Email notifications configured for:
  - [ ] New security vulnerabilities
  - [ ] Build failures
  - [ ] Coverage regressions
  - [ ] General bug reports

## Dashboard Access

- [ ] Bookmark bug dashboard: https://bugs.chromium.org/p/oss-fuzz/issues/list?q=label:Proj-mockforge
- [ ] Check coverage reports: https://coverage.fuzzbench.com/
- [ ] Review performance stats regularly

## Continuous Maintenance

- [ ] Set up weekly dashboard review
- [ ] Plan process for responding to security issues
- [ ] Document bug triage workflow
- [ ] Schedule quarterly review of fuzz targets
- [ ] Monitor corpus growth and performance

## Common Issues

### Build fails in CI but works locally
- Check OSS-Fuzz base image version
- Verify all dependencies are in Dockerfile
- Test with exact OSS-Fuzz environment

### Fuzz targets crash immediately
- Check for initialization issues
- Verify input validation is correct
- Review sanitizer output

### Low executions per second
- Profile fuzz targets
- Optimize initialization code
- Consider splitting complex targets

### Coverage not improving
- Review seed corpus quality
- Add more meaningful dictionary entries
- Check for unreachable code paths

## Resources

- [OSS-Fuzz Integration Guide](https://google.github.io/oss-fuzz/getting-started/new-project-guide/)
- [OSS-Fuzz FAQ](https://google.github.io/oss-fuzz/faq/)
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/)
- [Example Rust Projects](https://github.com/google/oss-fuzz/tree/master/projects)

## Notes

- Allow 1-2 weeks for initial review
- Be responsive to maintainer feedback
- Keep integration simple initially
- Can always add more fuzz targets later
- Focus on critical code paths first
