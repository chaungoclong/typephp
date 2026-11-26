<?php

declare(strict_types=1);
// Target: PHP 8.4

function factorial(int $n): int
{
    if ($n <= 1) {
        return 1;
    }
    return $n * factorial($n - 1);
}

/**
 * @return list<int>
 */
function collectNumbers(int $limit): array
{
    /** @var list<int> $items */
    $items = [];
    $i = 0;
    while ($i < $limit) {
        array_push($items, $i);
        $i = $i + 1;
    }
    return $items;
}

/** @var array{id: int, name: string, email?: string} $profile */
$profile = ['id' => 1, 'name' => 'Long'];
$email = $profile['email'] ?? 'none';
$fact5 = factorial(5);
/** @var list<int> $nums */
$nums = collectNumbers(3);
echo 'fact(5)=' . (string) $fact5 . ', count=' . (string) count($nums) . ', email=' . $email;
