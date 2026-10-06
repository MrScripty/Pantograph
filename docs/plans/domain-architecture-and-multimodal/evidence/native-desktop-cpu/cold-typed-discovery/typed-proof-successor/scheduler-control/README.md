# Native submission reaches a queued run; dependency control enters task schedule

[Run 37511768640](https://github.com/MrScripty/Pantograph/actions/runs/37511768640)
executes 62e3371b0c947fbf088853f9128404a547c36990, tree
b77ff948c9a5d7ed5990ca5d9677ea2f13d0be56. Native build, startup tests,
cold-owner, save/reopen, all three wires and real typed Resolve succeed.
GUI Submit clears snapshot publication and creates scoped queued run
run_e8ac33d3-60c0-45c9-aba8-8355830157bd. The scheduler rejects deps from
Invalid: its dependency control is incorrectly projected as an unsupported task.
The queued run has zero retained outputs and no runtime/device selection.
CPU execution/output are not established; this does not establish loader failure.

All twenty-two members, original images/JSON and masked job log are preserved.
Artifact 11435774716 SHA256 is
d8162747417705965ab6a666bbef3bedcb95ef353d72dc955d0192a503b772d4.
