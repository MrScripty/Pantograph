"""Independent ACK race regressions, additional to the frozen 13-case oracle.

Authored after source review identified the return-boundary races. Actual baseline
loaders, tiny CPU tensors and deterministic barriers exercise the producer; no
performance, inference quality, residency or scheduler authorization claim.
"""

from concurrent.futures import ThreadPoolExecutor
import gc
import json
from pathlib import Path
import threading
import time
import unittest
import weakref

import test_service_timing_owner_oracle as baseline


class NativeServiceTimingOwnerConcurrencyOracle(unittest.TestCase):
    setUp = baseline.NativeServiceTimingOwnerOracle.setUp
    load = baseline.NativeServiceTimingOwnerOracle.load
    capture = baseline.NativeServiceTimingOwnerOracle.capture

    def envelope(self, path, request_id):
        value = json.loads((baseline.TORCH_DIR.parent / "tests" / "fixtures" /
            "pytorch_worker_contract" / "load_transformers_model_request.json").read_text())
        value["request_id"] = request_id
        value["payload"]["entry_path"] = str(path)
        value["payload"]["model_source"]["entry_path"] = str(path)
        value["payload"]["device"] = "cpu"
        value["payload"]["trust_policy"]["revision"] = None
        return json.dumps(value)

    def test_foreign_actual_load_finishes_before_original_wrapped_ack(self):
        foreign = Path(self.directory.name) / "foreign"
        baseline.build_native_package(foreign)
        entered, resume = threading.Event(), threading.Event()
        original_info = self.worker.logger.info
        caller_thread = []
        intercepts = []

        def pause_after_actual_install(message, *args, **kwargs):
            if (message == "Model loaded: %s (%s)" and args[0] == self.package.name
                    and caller_thread and threading.get_ident() == caller_thread[0]):
                intercepts.append(True)
                entered.set()
                if not resume.wait(10):
                    raise RuntimeError("independent native ACK barrier timed out")
            return original_info(message, *args, **kwargs)

        def original_caller():
            caller_thread.append(threading.get_ident())
            return self.tracker.load_with_ack(self.envelope(self.package, "overlap-original"))

        self.worker.logger.info = pause_after_actual_install
        try:
            with ThreadPoolExecutor(max_workers=2) as pool:
                original = pool.submit(original_caller)
                try:
                    self.assertTrue(entered.wait(10), "actual original loader must reach post-install boundary")
                    # The native A model is installed, but its load_model wrapper
                    # has not returned/recorded ACK. Another actual caller installs B.
                    replacement = pool.submit(self.worker.load_model, str(foreign),
                        device="cpu", local_files_only=True, trust_remote_code=False)
                    self.assertEqual(replacement.result(timeout=10)["model_path"], str(foreign))
                    self.assertEqual(self.worker.get_loaded_info()["model_path"], str(foreign))
                finally:
                    resume.set()
                response, stamp = original.result(timeout=10)
        finally:
            resume.set()
            self.worker.logger.info = original_info
        self.assertEqual(intercepts, [True], "controlled actual install/return gap must execute")
        self.assertEqual(json.loads(response)["status"], "ok", "observation refusal must preserve native response")
        self.assertIsNone(stamp, "original successful response must not attach the foreign installed owner")
        self.assertEqual(self.worker.get_loaded_info()["model_path"], str(foreign))
        # Refusing this attempt does not permanently disable otherwise valid owners.
        self.load()
        self.capture()

    def test_two_same_thread_load_acks_cannot_pair_second_owner_with_first_response(self):
        foreign = Path(self.directory.name) / "second"
        baseline.build_native_package(foreign)
        original_facade = self.worker.load_transformers_model_from_envelope
        calls = []

        def return_first_after_second(raw):
            first = original_facade(raw)
            second = original_facade(self.envelope(foreign, "same-thread-second"))
            self.assertEqual(json.loads(second)["status"], "ok")
            calls.append((threading.get_ident(), self.worker.get_loaded_info()["model_path"]))
            return first

        self.worker.load_transformers_model_from_envelope = return_first_after_second
        try:
            response, stamp = self.tracker.load_with_ack(self.envelope(self.package, "same-thread-first"))
        finally:
            self.worker.load_transformers_model_from_envelope = original_facade
        self.assertEqual(calls, [(threading.get_ident(), str(foreign))])
        self.assertEqual(json.loads(response)["status"], "ok")
        self.assertIsNone(stamp, "multiple ACKs in one envelope scope cannot certify which native result was returned")
        self.assertEqual(self.worker.get_loaded_info()["model_path"], str(foreign))
        fresh_response, fresh = self.tracker.load_with_ack(self.envelope(self.package, "single-ack-again"))
        self.assertEqual(json.loads(fresh_response)["status"], "ok")
        self.assertIsNotNone(fresh)
        self.assertEqual(self.tracker.revalidate(fresh), fresh.facts_json)


    def test_generation_original_hash_controls_real_default_resolution(self):
        self.load()
        model = self.worker._model
        model.config.max_length = 9
        model.generation_config._from_model_config = True
        model.generation_config._original_object_hash = hash(model.generation_config)
        initial, before = self.capture()
        model.generation_config._original_object_hash += 1
        self.assertIsNone(self.tracker.revalidate(initial),
            "native generation hash predicate is executable state, not a locator label")
        _changed, after = self.capture()
        self.assertNotEqual(before["effective_configuration_fingerprint"],
                            after["effective_configuration_fingerprint"])
        # Invoke the actual installed Transformers branch, not a reference copy.
        unchanged, _kwargs = model._prepare_generation_config(None)
        self.assertEqual(unchanged.max_length, 20)
        model.generation_config._original_object_hash = hash(model.generation_config)
        inherited, _kwargs = model._prepare_generation_config(None)
        self.assertEqual(inherited.max_length, 9)



    def test_shared_shutdown_waits_for_actual_capture_and_releases_temporary_native_refs(self):
        self.load()
        old, _facts = self.capture()
        held_model = weakref.ref(self.worker._model)
        held_tokenizer = weakref.ref(self.worker._tokenizer)
        paused, resume = threading.Event(), threading.Event()
        original_model_state = baseline.owner._model_state
        phase_entries = []

        def pause_actual_model_hash(model, native_torch):
            result = original_model_state(model, native_torch)
            phase_entries.append(True)
            paused.set()
            if not resume.wait(10):
                raise RuntimeError("independent native collector barrier timed out")
            return result

        baseline.owner._model_state = pause_actual_model_hash
        try:
            with ThreadPoolExecutor(max_workers=2) as pool:
                collection = pool.submit(self.tracker.capture)
                try:
                    self.assertTrue(paused.wait(10), "actual native hash must reach controlled barrier")
                    shutdown = pool.submit(self.worker.shutdown_worker)
                    # Observe the actual instrumented transition, not merely a
                    # caller announcing that it intends to invoke shutdown.
                    deadline = time.monotonic() + 10
                    began = False
                    while time.monotonic() < deadline:
                        with self.tracker._state_lock:
                            began = self.tracker._busy > 0
                        if began:
                            break
                        time.sleep(0.001)
                    self.assertTrue(began, "actual shared-worker shutdown transition must begin")
                    self.assertFalse(shutdown.done(), "physical stop ACK must wait for held actual collector")
                    self.assertFalse(resume.is_set())
                    self.assertIsNotNone(held_model())
                    self.assertIsNotNone(held_tokenizer())
                    self.assertIs(self.worker._model, held_model(), "original shutdown must not remove model while hash owns it")
                    self.assertIsNone(self.tracker.revalidate(old), "transition invalidates old evidence before drain")
                finally:
                    resume.set()
                self.assertIsNone(collection.result(timeout=10), "intervening shutdown refuses in-flight identity")
                self.assertIsNone(shutdown.result(timeout=10))
        finally:
            resume.set()
            baseline.owner._model_state = original_model_state
        self.assertEqual(phase_entries, [True])
        self.assertIsNone(self.worker.get_loaded_info())
        gc.collect()
        self.assertIsNone(held_model(), "finished collector must not retain stopped model")
        self.assertIsNone(held_tokenizer(), "finished collector must not retain stopped tokenizer")
        self.assertIsNone(self.tracker.capture())
        self.assertIsNone(self.tracker.revalidate(old))



    def test_oversized_native_unknown_token_absent_from_vocab_refuses_before_export(self):
        self.load()
        old, _facts = self.capture()
        native = self.worker._tokenizer.backend_tokenizer
        vocabulary = native.get_vocab(with_added_tokens=True)
        self.assertEqual(len(vocabulary), 8, "keep actual vocabulary inside native profile")
        unknown = "x" * (baseline.owner.MAX_STRING + 1)
        self.assertNotIn(unknown, vocabulary, "unknown-token field is not covered by vocab preflight")
        native.model = baseline.models.WordLevel(vocabulary, unk_token=unknown)
        self.assertEqual(native.model.unk_token, unknown)
        # This payload is below the prior native-export limit, so an export-size
        # refusal cannot stand in for checking the independent unknown-token field.
        serialized = native.to_str(pretty=False)
        self.assertLess(len(serialized.encode("utf-8")), baseline.owner.MAX_TOKENIZER_BYTES)
        self.assertIsNone(self.tracker.capture(),
            "oversized native WordLevel unknown-token metadata must refuse evidence")
        self.assertIsNone(self.tracker.revalidate(old), "old identity cannot certify changed tokenizer")
        self.load()
        self.capture()


if __name__ == "__main__":
    unittest.main()
