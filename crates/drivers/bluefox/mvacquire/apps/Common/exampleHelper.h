//-----------------------------------------------------------------------------
#ifndef exampleHelperH
#define exampleHelperH exampleHelperH
//-----------------------------------------------------------------------------
#include <algorithm>
#include <iostream>
#include <set>
#include <string>
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>

//-----------------------------------------------------------------------------
template<class _Ty>
class DisplayDictEntry : public std::unary_function<std::pair<std::string, _Ty>, void>
//-----------------------------------------------------------------------------
{
public:
    void operator()( const std::pair<std::string, _Ty>& data ) const
    {
        std::cout << "  [" << data.second << "]: " << data.first << std::endl;
    }
};

//-----------------------------------------------------------------------------
class DisplayProperty : public std::unary_function<std::pair<std::string, mvIMPACT::acquire::Property>, void>
//-----------------------------------------------------------------------------
{
public:
    void operator()( const std::pair<std::string, mvIMPACT::acquire::Property>& data ) const
    {
        if( data.second.isValid() )
        {
            std::cout << data.first << ": " << data.second.readSArray() << "(" << data.second.flagsAsString() << ")" << std::endl;
        }
    }
};

//-----------------------------------------------------------------------------
template<class _Ty>
void DisplayPropertyDictionary( const mvIMPACT::acquire::Property& p )
//-----------------------------------------------------------------------------
{
    _Ty prop( p );
#if defined(_MSC_VER) && (_MSC_VER < 1300) // is 'old' VC 6 Microsoft compiler?
    std::vector<std::pair<std::string, _Ty::value_type> > dict;
    prop.getTranslationDict( dict );
    std::for_each( dict.begin(), dict.end(), DisplayDictEntry<_Ty::value_type>() );
#else
    std::vector<std::pair<std::string, typename _Ty::value_type> > dict;
    prop.getTranslationDict( dict );
    std::for_each( dict.begin(), dict.end(), DisplayDictEntry<typename _Ty::value_type>() );
#endif // #ifdef _MSC_VER
}

//-----------------------------------------------------------------------------
/// \brief Checks is a certain value for property is supported.
template<class _Tx, typename _Ty>
bool supportsValue( const _Tx& prop, const _Ty& value )
//-----------------------------------------------------------------------------
{
    if( prop.hasDict() )
    {
        typename std::vector<_Ty> sequence;
        prop.getTranslationDictValues( sequence );
        return std::find( sequence.begin(), sequence.end(), value ) != sequence.end();
    }

    if( prop.hasMinValue() && ( prop.getMinValue() > value ) )
    {
        return false;
    }

    if( prop.hasMaxValue() && ( prop.getMaxValue() < value ) )
    {
        return false;
    }

    return true;
}

//-----------------------------------------------------------------------------
/// \brief Sets a property to a certain value if this value is supported.
template<typename _Ty, typename _Tx>
void conditionalSetProperty( const _Ty& prop, const _Tx& value )
//-----------------------------------------------------------------------------
{
    if( prop.isValid() && supportsValue( prop, value ) )
    {
        prop.write( value );
        std::cout << "Property '" << prop.name() << "' set to '" << prop.readS() << "'." << std::endl;
    }
}

//-----------------------------------------------------------------------------
/// This function makes heavy use of strings. In real world applications
/// this can be avoided if optimal performance is crucial. All properties can be modified
/// via strings, but most properties can also be modified with numerical (int / double )
/// values, which is much faster, but not as descriptive for a sample application
inline void displayPropertyData( const mvIMPACT::acquire::Property& prop )
//-----------------------------------------------------------------------------
{
    const std::string name( prop.name() );
    std::cout << "Property '" << name << "' currently specifies the following flags: " << prop.flagsAsString() << std::endl;
    if( prop.hasMinValue() )
    {
        std::cout << "The minimum value of '" << name << "' is " << prop.readS( mvIMPACT::acquire::plMinValue ) << std::endl;
    }
    if( prop.hasMaxValue() )
    {
        std::cout << "The maximum value of '" << name << "' is " << prop.readS( mvIMPACT::acquire::plMaxValue ) << std::endl;
    }
    if( prop.hasStepWidth() )
    {
        std::cout << "The increment of '" << name << "' is " << prop.readS( mvIMPACT::acquire::plStepWidth ) << std::endl;
    }
    if( prop.hasDict() )
    {
        std::cout << "'" << name << "' defines a dictionary. Valid values are: " << std::endl;
        mvIMPACT::acquire::TComponentType type = prop.type();
        if( type == mvIMPACT::acquire::ctPropInt )
        {
            DisplayPropertyDictionary<mvIMPACT::acquire::PropertyI>( prop );
        }
        else if( type == mvIMPACT::acquire::ctPropInt64 )
        {
            DisplayPropertyDictionary<mvIMPACT::acquire::PropertyI64>( prop );
        }
        else if( type == mvIMPACT::acquire::ctPropFloat )
        {
            DisplayPropertyDictionary<mvIMPACT::acquire::PropertyF>( prop );
        }
        else
        {
            std::cout << "Error! Unhandled enum prop type: " << prop.typeAsString() << std::endl;
        }
    }
    std::cout << "The current value of '" << name << "' is: " << prop.readS() << std::endl;
}

//-----------------------------------------------------------------------------
inline bool displayPropertyDataWithValidation( const mvIMPACT::acquire::Property& prop, const std::string& name )
//-----------------------------------------------------------------------------
{
    if( !prop.isValid() )
    {
        std::cout << "Property '" << name << "' is not supported/available." << std::endl;
        return false;
    }
    displayPropertyData( prop );
    return true;
}

//-----------------------------------------------------------------------------
/// This function makes heavy use of strings. In real world applications
/// this can be avoided if optimal performance is crucial. All properties can be modified
/// via strings, but most properties can also be modified with numerical (int / double )
/// values, which is much faster, but not as descriptive for a sample application
inline void modifyPropertyValue( const mvIMPACT::acquire::Property& prop, const std::string& param = "", const std::string& index = "" )
//-----------------------------------------------------------------------------
{
    try
    {
        const std::string name( prop.name() );
        int valIndex = 0;
        if( prop.isWriteable() )
        {
            if( param.empty() )
            {
                std::cout << "Enter the new value for '" << name << "': ";
                std::string val;
                std::cin >> val;
                // remove the '\n' from the stream
                std::cin.get();
                if( prop.valCount() > 1 )
                {
                    std::cout << "'" << name << "' defines " << prop.valCount() << " values. Enter the index (zero-based) of the value to modify: ";
                    std::cin >> valIndex;
                    // remove the '\n' from the stream
                    std::cin.get();
                }
                prop.writeS( val, valIndex );
            }
            else
            {
                if( !index.empty() )
                {
                    valIndex = atoi( index.c_str() );
                }
                prop.writeS( param, valIndex );
            }
        }
        else
        {
            std::cout << "'" << name << "' is read-only, thus can't be modified." << std::endl;
        }
    }
    catch( const mvIMPACT::acquire::ImpactAcquireException& e )
    {
        std::cout << "An exception occurred: " << e.getErrorString() << "(error code: " << e.getErrorCodeAsString() << ")" << std::endl;
    }
}

//-----------------------------------------------------------------------------
inline void displayAndModifyPropertyDataWithValidation( const mvIMPACT::acquire::Property& prop, const std::string& name )
//-----------------------------------------------------------------------------
{
    if( displayPropertyDataWithValidation( prop, name ) )
    {
        modifyPropertyValue( prop );
    }
    std::cout << std::endl;
}

//-----------------------------------------------------------------------------
inline std::ostream& operator<<( std::ostream& out, const mvIMPACT::acquire::Property& prop )
//-----------------------------------------------------------------------------
{
    out << prop.name() << ": " << prop.readS();
    return out;
}

//-----------------------------------------------------------------------------
/// \brief Allows string comparison with a defined character to ignore
///
/// This function allows a tolerant string compare. If \a candidate ends with \a wildcard
/// \a candidate can be shorter then \a searchString as the rest of the string will be
/// ignored. This is a helper function used internally by <b>DeviceManager</b> objects.
///
/// Examples:
///
///\code
/// wildcard = '*'
/// s1 = "blablabla"
/// match( s1, "bl*bl*bla", '*' ); // will return 0
/// // will return 0 ('*' is the default value for parameter 3 )
/// match( s1, "bl*" );
/// // the next call will return -1 as the first character MUST
/// // be either a 'b' or the wildcard character.
/// match( s1, "a*" );
///\endcode
/// \return
/// - 0 if successful
/// - -1 otherwise
template<class _Elem, class _Traits, class _Ax>
int match( const std::basic_string<_Elem, _Traits, _Ax>& searchString, const std::basic_string<_Elem, _Traits, _Ax>& candidate, _Elem wildcard )
//-----------------------------------------------------------------------------
{
    typename std::basic_string<_Elem, _Traits, _Ax>::size_type searchLength = searchString.length();
    // determine search length
    if( candidate.length() < searchString.length() )
    {
        if( candidate.empty() )
        {
            return -1;
        }

        if( candidate[candidate.length() - 1] != wildcard )
        {
            return -1;
        }
        searchLength = candidate.length() - 1;
    }
    // search
    for( typename std::basic_string<_Elem, _Traits, _Ax>::size_type i = 0; i < searchLength; i++ )
    {
        if( ( candidate[i] != searchString[i] ) && ( candidate[i] != wildcard ) )
        {
            return -1;
        }
    }
    return 0;
}

//-----------------------------------------------------------------------------
/// The 'Generic' interface layout has been declared deprecated. However as some
/// of the exisiting applications out there might still depend on it, we can't
/// silently switch to the 'GenICam' layout. Thus we will do this for our samples
/// only at this time.
inline void switchFromGenericToGenICamInterface( Device* pDev )
//-----------------------------------------------------------------------------
{
    if( !pDev->interfaceLayout.isValid() )
    {
        return;
    }

    try
    {
        if( pDev->interfaceLayout.readS() == "Generic" )
        {
            pDev->interfaceLayout.write( dilGenICam );
        }
    }
    catch( const ImpactAcquireException& ) {}
}

typedef bool( *SUPPORTED_DEVICE_CHECK )( const mvIMPACT::acquire::Device* const );

//-----------------------------------------------------------------------------
inline mvIMPACT::acquire::Device* getDeviceFromUserInput( const mvIMPACT::acquire::DeviceManager& devMgr, SUPPORTED_DEVICE_CHECK pSupportedDeviceCheckFn = 0, bool boSilent = false )
//-----------------------------------------------------------------------------
{
    const unsigned int devCnt = devMgr.deviceCount();
    if( devCnt == 0 )
    {
        std::cout << "No compatible device found!" << std::endl;
        return 0;
    }

    std::set<unsigned int> validDeviceNumbers;

    // display every device detected that matches
    for( unsigned int i = 0; i < devCnt; i++ )
    {
        Device* pDev = devMgr[i];
        if( pDev )
        {
            if( !pSupportedDeviceCheckFn || pSupportedDeviceCheckFn( pDev ) )
            {
                switchFromGenericToGenICamInterface( pDev );
                std::cout << "[" << i << "]: " << pDev->serial.read() << " (" << pDev->product.read() << ", " << pDev->family;
                if( pDev->interfaceLayout.isValid() )
                {
                    std::cout << ", interface layout: " << pDev->interfaceLayout.readS();
                }
                std::cout << ")" << std::endl;
                validDeviceNumbers.insert( i );
            }
        }
    }

    if( validDeviceNumbers.empty() )
    {
        std::cout << devMgr.deviceCount() << " devices have been detected:" << std::endl;
        for( unsigned int i = 0; i < devCnt; i++ )
        {
            Device* pDev = devMgr[i];
            if( pDev )
            {
                std::cout << "  [" << i << "]: " << pDev->serial.read() << " (" << pDev->product.read() << ", " << pDev->family << ")" << std::endl;
            }
        }
        std::cout << "However none of these devices seems to be supported by this sample." << std::endl << std::endl;
        return 0;
    }

    // get user input
    std::cout << std::endl << "Please enter the number in front of the listed device followed by [ENTER] to open it: ";
    unsigned int devNr = 0;
    std::cin >> devNr;
    // remove the '\n' from the stream
    std::cin.get();

    if( validDeviceNumbers.find( devNr ) == validDeviceNumbers.end() )
    {
        std::cout << "Invalid selection!" << std::endl;
        return 0;
    }

    if( !boSilent )
    {
        std::cout << "Using device number " << devNr << "." << std::endl;
    }
    return devMgr[devNr];
}

//-----------------------------------------------------------------------------
inline std::vector<mvIMPACT::acquire::Device*>::size_type getValidDevices( const mvIMPACT::acquire::DeviceManager& devMgr, std::vector<mvIMPACT::acquire::Device*>& v, SUPPORTED_DEVICE_CHECK pSupportedDeviceCheckFn = 0 )
//-----------------------------------------------------------------------------
{
    const unsigned int devCnt = devMgr.deviceCount();
    // display every device detected that matches
    for( unsigned int i = 0; i < devCnt; i++ )
    {
        Device* pDev = devMgr[i];
        if( pDev )
        {
            if( !pSupportedDeviceCheckFn || pSupportedDeviceCheckFn( pDev ) )
            {
                v.push_back( pDev );
            }
        }
    }
    return v.size();
}

#ifdef linux
#   include <fcntl.h>
#   include <stdio.h>
#   include <sys/types.h>
#   include <termios.h>
#   include <unistd.h>
//-----------------------------------------------------------------------------
/// \brief Checks if something was written to a certain file descriptor
/** \return
 *  - 0 if timeout did elapse
 *  - 1 otherwise
 */
inline unsigned waitForInput( int maxWait_sec, int fd )
//-----------------------------------------------------------------------------
{
    fd_set rfds;
    struct timeval tv;

    FD_ZERO( &rfds );
    FD_SET( fd, &rfds );

    tv.tv_sec = maxWait_sec ;
    tv.tv_usec = 0;

    return select( fd + 1, &rfds, NULL, NULL, &tv );
}

//-----------------------------------------------------------------------------
/** \return
 *  - 1 if a key has been pressed since the last call to this function
 *  - 0 otherwise
 */
inline int checkKeyboardInput( void )
//-----------------------------------------------------------------------------
{
    struct termios oldt, newt;
    int ch;
    int oldf;

    tcgetattr( STDIN_FILENO, &oldt );
    newt = oldt;
    newt.c_lflag &= ~( ICANON | ECHO );
    tcsetattr( STDIN_FILENO, TCSANOW, &newt );
    oldf = fcntl( STDIN_FILENO, F_GETFL, 0 );
    fcntl( STDIN_FILENO, F_SETFL, oldf | O_NONBLOCK );

    ch = getchar();

    tcsetattr( STDIN_FILENO, TCSANOW, &oldt );
    fcntl( STDIN_FILENO, F_SETFL, oldf );

    if( ch != EOF )
    {
        //ungetc(ch, stdin);
        return 1;
    }

    return 0;
}
#endif // #ifdef linux

#endif // exampleHelperH
