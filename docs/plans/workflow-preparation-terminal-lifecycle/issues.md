# Issues

- L-01 (resolved locally): registered worker preparation refusal now uses guarded
  canonical workflow finish/terminal. Source a8a9f105 passes one terminal,
  released session, controlled zero native/Start work and public close proof.
- L-02 (deferred, existing): post-Start/assigned failures may need task/reservation/
  batch cleanup; never finish them through this zero-active-custody repair. Parent
  and cleanup owner must admit a separate lifecycle slice if reproduction needs it.
- L-03 (deferred, existing): admitted API failures before worker enqueue and worker
  supervision exits bypass this driver. Owner session_execution_api/supervision;
  revisit if parent expands lifecycle scope. No broad all-admission claim here.
- M4 (blocked): the parent approved/published PR83 at 5b4fc6dd; its public
  preparation hook is source-confirmed to use this unchanged worker driver.
  Actual V2 rerun/acceptance still requires parent/owner prospective protocol
  coordination. No actual cohort or native acceptance is claimed here.
