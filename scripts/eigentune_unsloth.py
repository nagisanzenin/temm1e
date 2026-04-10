#!/usr/bin/env python3
"""Eigen-Tune Unsloth wrapper.

Vendored Python driver invoked by the Rust unsloth backend
(crates/temm1e-distill/src/backends/unsloth.rs). Reads ChatML JSONL
training data from a directory containing train.jsonl and (optionally)
valid.jsonl, fine-tunes a base model with LoRA via Unsloth's
FastLanguageModel + TRL's SFTTrainer, and writes the adapter to disk.

Required Python packages:
    pip install unsloth trl datasets

The wrapper is intentionally minimal: it accepts a small set of CLI args,
prints progress to stdout (which the Rust caller streams to tracing), and
emits a single line `EIGENTUNE_RESULT {json}` at the end with parseable
metrics for the caller.

This script is NEVER executed automatically — only invoked when the
Rust backend resolves `select_backend("unsloth"|"auto")` to UnslothBackend.
"""

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Eigen-Tune Unsloth fine-tuning wrapper")
    parser.add_argument("--model", required=True, help="HuggingFace repo ID or local path of the base model")
    parser.add_argument("--data", required=True, help="Directory with train.jsonl and (optionally) valid.jsonl")
    parser.add_argument("--output", required=True, help="Directory where adapter weights will be written")
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--lr", type=float, default=2e-4)
    parser.add_argument("--lora-r", type=int, default=32)
    parser.add_argument("--lora-alpha", type=int, default=64)
    parser.add_argument("--batch-size", type=int, default=4)
    parser.add_argument("--max-seq-len", type=int, default=4096)
    args = parser.parse_args()

    data_dir = Path(args.data)
    train_jsonl = data_dir / "train.jsonl"
    if not train_jsonl.exists():
        print(f"ERROR: train.jsonl not found in {data_dir}", file=sys.stderr)
        return 1

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Imports are lazy so the script can at least show its --help without
    # the heavy ML dependencies installed.
    try:
        from unsloth import FastLanguageModel  # type: ignore
        from trl import SFTTrainer  # type: ignore
        from transformers import TrainingArguments  # type: ignore
        from datasets import load_dataset  # type: ignore
    except ImportError as e:
        print(f"ERROR: required dependency missing: {e}", file=sys.stderr)
        print("Install with: pip install unsloth trl datasets transformers", file=sys.stderr)
        return 2

    print(f"Loading base model: {args.model}")
    model, tokenizer = FastLanguageModel.from_pretrained(
        args.model,
        max_seq_length=args.max_seq_len,
        dtype=None,
        load_in_4bit=True,
    )
    model = FastLanguageModel.get_peft_model(
        model,
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        target_modules=[
            "q_proj", "k_proj", "v_proj", "o_proj",
            "gate_proj", "up_proj", "down_proj",
        ],
        use_gradient_checkpointing="unsloth",
    )

    print(f"Loading dataset from: {data_dir}")
    train_ds = load_dataset("json", data_files=str(train_jsonl), split="train")
    valid_jsonl = data_dir / "valid.jsonl"
    eval_ds = None
    if valid_jsonl.exists():
        eval_ds = load_dataset("json", data_files=str(valid_jsonl), split="train")

    def to_text(example):
        return {
            "text": tokenizer.apply_chat_template(
                example["messages"], tokenize=False
            )
        }

    train_ds = train_ds.map(to_text)
    if eval_ds is not None:
        eval_ds = eval_ds.map(to_text)

    training_args = TrainingArguments(
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=4,
        num_train_epochs=args.epochs,
        learning_rate=args.lr,
        output_dir=str(output_dir),
        save_strategy="epoch",
        logging_steps=10,
        optim="adamw_8bit",
        report_to="none",
    )

    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=train_ds,
        eval_dataset=eval_ds,
        dataset_text_field="text",
        max_seq_length=args.max_seq_len,
        args=training_args,
    )

    print("Starting training...")
    result = trainer.train()
    print(f"Training complete. Final loss: {result.training_loss:.4f}")

    # Save adapter weights as adapter_model.safetensors in the output dir
    model.save_pretrained(str(output_dir))
    tokenizer.save_pretrained(str(output_dir))

    summary = {
        "train_loss": float(result.training_loss),
        "epochs_completed": int(args.epochs),
    }
    print(f"EIGENTUNE_RESULT {json.dumps(summary)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
