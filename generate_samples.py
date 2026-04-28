import argparse
import os
import subprocess


def run_ffmpeg(cmd):
    try:
        subprocess.run(
            cmd,
            check=True,
            shell=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError as e:
        print(f"Error running command: {e}")


def create_variants(input_a, output_dir, input_b=None):
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)

    # 1. Create Base/Clean Mix
    base_file = os.path.join(output_dir, "00_clean_target.mp3")
    if input_b:
        print(f"Mixing {input_a} and {input_b}...")
        # Mixes two tracks equally and takes 15 seconds
        cmd = f'ffmpeg -y -i "{input_a}" -i "{input_b}" -filter_complex "amix=inputs=2:duration=first" -ss 30 -t 15 {base_file}'
    else:
        print(f"Trimming {input_a}...")
        cmd = f'ffmpeg -y -i "{input_a}" -ss 30 -t 15 {base_file}'
    run_ffmpeg(cmd)

    # 2. Variant: Heavy White Noise (Tests SNR robustness)
    print("Generating Noise variant...")
    run_ffmpeg(
        f'ffmpeg -y -i {base_file} -f lavfi -i "anoisesrc=a=0.2:c=white" -filter_complex "[0:a][1:a]amix=inputs=2:duration=first" {output_dir}/var_noise.mp3'
    )

    # 3. Variant: High Pitch / Speed Up (Tests time-scaling robustness)
    print("Generating Speed variant...")
    run_ffmpeg(f'ffmpeg -y -i {base_file} -af "atempo=1.10" {output_dir}/var_fast.mp3')

    # 4. Variant: Low Pass Filter (Simulates muffled speaker/distance)
    print("Generating Lowpass variant...")
    run_ffmpeg(
        f'ffmpeg -y -i {base_file} -af "lowpass=f=1000" {output_dir}/var_muffled.mp3'
    )

    # 5. Variant: Radio/Phone Effect (Bandpass + Compression)
    print("Generating Telephonic variant...")
    run_ffmpeg(
        f'ffmpeg -y -i {base_file} -af "highpass=f=300, lowpass=f=3000, volume=1.5" {output_dir}/var_phone.mp3'
    )
    # 5. Variant: Radio/Phone Effect (Bandpass + Compression)
    print("Generating Worst variant...")
    run_ffmpeg(
        f'ffmpeg -y -i "{base_file}" -f lavfi -i "anoisesrc=a=0.1:c=white" -filter_complex '
        f'"[1:a]lowpass=f=3000,highpass=f=300,volume=1.5[noise]; '
        f"[0:a]atempo=1.10[music]; "
        f"[music][noise]amix=inputs=2:duration=first:dropout_transition=0[mixed]; "
        f'[mixed]aecho=0.8:0.88:60:0.4,highpass=f=200,lowpass=f=3500,volume=0.8" '
        f'"{output_dir}/shazam_test_bar.mp3"'
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generate Audio Fingerprinting Test Samples"
    )
    parser.add_argument("song1", help="Path to first song")
    parser.add_argument(
        "--song2", help="Path to second song (optional mix mode)", default=None
    )
    parser.add_argument("--out", help="Output folder", default="test_samples")

    args = parser.parse_args()
    create_variants(args.song1, args.out, args.song2)
    print(f"\nDone! Samples generated in ./{args.out}")
