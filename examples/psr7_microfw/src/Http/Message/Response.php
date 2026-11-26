<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/MessageInterface.php';
require_once __DIR__ . '/ResponseInterface.php';
require_once __DIR__ . '/StreamInterface.php';

class Response implements ResponseInterface
{
    private int $statusCode;
    private string $reasonPhrase;
    /** @var array<string, string> */
    private array $headers;
    private StreamInterface $body;

    /**
     * @param array<string, string> $headers
     */
    public function __construct(int $statusCode, array $headers, StreamInterface $body, string $reasonPhrase = '')
    {
        $this->statusCode = $statusCode;
        $this->headers = $headers;
        $this->body = $body;
        $this->reasonPhrase = $reasonPhrase;
    }

    public function getProtocolVersion(): string
    {
        return '1.1';
    }

    public function getStatusCode(): int
    {
        return $this->statusCode;
    }

    public function getReasonPhrase(): string
    {
        return $this->reasonPhrase;
    }

    /**
     * @return array<string, string>
     */
    public function getHeaders(): array
    {
        return $this->headers;
    }

    public function hasHeader(string $name): bool
    {
        return array_key_exists(strtolower($name), $this->headers);
    }

    public function getHeaderLine(string $name): string
    {
        $key = strtolower($name);
        if (array_key_exists($key, $this->headers)) {
            return $this->headers[$key];
        }
        return '';
    }

    public function withHeader(string $name, string $value): MessageInterface
    {
        /** @var array<string, string> $next */
        $next = $this->headers;
        $next[strtolower($name)] = $value;
        return new Response($this->statusCode, $next, $this->body, $this->reasonPhrase);
    }

    public function getBody(): StreamInterface
    {
        return $this->body;
    }

    public function withBody(StreamInterface $body): MessageInterface
    {
        return new Response($this->statusCode, $this->headers, $body, $this->reasonPhrase);
    }

    public function withStatus(int $code, string $reasonPhrase = ''): ResponseInterface
    {
        return new Response($code, $this->headers, $this->body, $reasonPhrase);
    }
}
