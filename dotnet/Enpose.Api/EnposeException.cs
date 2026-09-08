using System;

namespace Enpose
{
    /// <summary>
    /// Thrown when a call into the Enpose library fails — an I/O failure
    /// during discovery, a stream that could not be connected, or an
    /// unrecoverable error while receiving poses.
    /// </summary>
    public class EnposeException : Exception
    {
        /// <summary>Create an exception with the given message.</summary>
        /// <param name="message">Description of the failure.</param>
        public EnposeException(string message) : base(message)
        {
        }

        /// <summary>Create an exception with the given message and cause.</summary>
        /// <param name="message">Description of the failure.</param>
        /// <param name="innerException">The underlying cause.</param>
        public EnposeException(string message, Exception innerException)
            : base(message, innerException)
        {
        }
    }
}
