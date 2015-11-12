//-----------------------------------------------------------------------------
#ifndef DataConversionH
#define DataConversionH DataConversionH
//-----------------------------------------------------------------------------
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include <string>
#include <wx/wx.h>
#include <wx/colour.h>

class PropData;

typedef std::map<wxString, PropData*> NameToFeatureMap;
typedef std::map<PropData*, wxString> FeatureToNameMap;

#if defined(linux) && ( defined(__x86_64__) || defined(__powerpc64__) ) // -m64 makes GCC define __powerpc64__
#   define MY_FMT_I64 "%ld"
#   define MY_FMT_I64_0_PADDED "%020ld"
#else
#   define MY_FMT_I64 "%lld"
#   define MY_FMT_I64_0_PADDED "%020lld"
#endif // #if defined(linux) && ( defined(__x86_64__) || defined(__powerpc64__) ) // -m64 makes GCC define __powerpc64__

std::string charToFormat( int c );
std::string charToType( int c );

//-----------------------------------------------------------------------------
/// \brief This is a singleton class
class GlobalDataStorage
//-----------------------------------------------------------------------------
{
private:
    mvIMPACT::acquire::TComponentVisibility componentVisibility_;
    bool boComponentVisibilitySupported_;
    const wxColour LIST_BACKGROUND_COLOUR_;
    const wxColour IS_DEFAULT_VALUE_COLOUR_;
    const wxColour INVISIBLE_GURU_FEATURE_COLOUR_;
    static GlobalDataStorage* pInstance_;
    explicit GlobalDataStorage();
public:
    ~GlobalDataStorage();
    static GlobalDataStorage* Instance( void );
    mvIMPACT::acquire::TComponentVisibility GetComponentVisibility( void ) const
    {
        return componentVisibility_;
    }
    void SetComponentVisibility( mvIMPACT::acquire::TComponentVisibility componentVisibility )
    {
        componentVisibility_ = componentVisibility;
    }

    void ConfigureComponentVisibilitySupport( bool boIsSupported )
    {
        boComponentVisibilitySupported_ = boIsSupported;
    }
    bool IsComponentVisibilitySupported( void ) const
    {
        return boComponentVisibilitySupported_;
    }

    //-----------------------------------------------------------------------------
    enum TPropGridColour
    //-----------------------------------------------------------------------------
    {
        pgcListBackground,
        pgcIsDefaultValue,
        pgcInvisibleExpertFeature,
        pgcInvisibleGuruFeature,
        pgcInvisibleFeature
    };
    const wxColour& GetPropGridColour( TPropGridColour colour ) const;

    NameToFeatureMap nameToFeatureMap_;
    FeatureToNameMap featureToNameMap_;
};

#endif // DataConversionH
