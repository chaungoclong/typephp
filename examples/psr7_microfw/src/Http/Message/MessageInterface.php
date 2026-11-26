<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/StreamInterface.php';

interface MessageInterface
{
    public function getProtocolVersion(): string;
    /**
     * @return array<string, string>
     */
    public function getHeaders(): array;
    public function hasHeader(string $name): bool;
    public function getHeaderLine(string $name): string;
    public function withHeader(string $name, string $value): MessageInterface;
    public function getBody(): StreamInterface;
    public function withBody(StreamInterface $body): MessageInterface;
}
