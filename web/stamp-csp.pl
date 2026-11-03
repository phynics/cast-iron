#!/usr/bin/perl
# Trunk post-build hook. Trunk writes its start-up script and our first-paint style inline,
# and a strict Content-Security-Policy allows an inline block only by hash. This computes the
# hashes of the finished index.html and fills in the placeholders of the policy.
use strict;
use warnings;
use Digest::SHA qw(sha256);
use MIME::Base64 qw(encode_base64);

my $dir  = $ENV{TRUNK_STAGING_DIR} or die "TRUNK_STAGING_DIR is not set\n";
my $path = "$dir/index.html";
open my $in, '<:raw', $path or die "cannot read $path: $!\n";
my $html = do { local $/; <$in> };
close $in;

sub digest { return 'sha256-' . encode_base64(sha256($_[0]), '') }

for my $block (['script', '__CSP_SCRIPT_HASH__'], ['style', '__CSP_STYLE_HASH__']) {
    my ($tag, $placeholder) = @$block;
    my @bodies = $html =~ m{<$tag(?:\s[^>]*)?>(.*?)</$tag>}sg;
    die "expected exactly one inline <$tag>, found " . scalar(@bodies) . "\n" unless @bodies == 1;
    my $hash = digest($bodies[0]);
    $html =~ s/\Q$placeholder\E/$hash/ or die "$placeholder is missing from index.html\n";
}

open my $out, '>:raw', $path or die "cannot write $path: $!\n";
print $out $html;
close $out;
